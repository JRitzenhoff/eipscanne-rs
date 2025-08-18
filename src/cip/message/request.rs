use std::io::{Read, Seek, SeekFrom};
use std::mem;

use binrw::{
    binrw, binwrite, BinRead, BinResult, BinWrite
};

use super::shared::{ServiceCode, ServiceContainer};
use crate::cip::path::CipPath;
use crate::cip::types::CipUsint;

#[binrw::writer(writer, endian)]
fn write_cip_path_with_size(cip_path: &CipPath) -> BinResult<()> {
    // Step 1: Write the `cip_path` field
    let mut temp_buffer = Vec::new();
    let mut temp_writer = std::io::Cursor::new(&mut temp_buffer);

    cip_path.write_options(&mut temp_writer, endian, ())?;

    // Step 2: Calculate the `cip_path` byte size
    let cip_path_word_size = (temp_buffer.len()) / mem::size_of::<u16>();

    // Step 3: Write the full struct
    if let Err(write_err) = writer.write(&[cip_path_word_size as CipUsint]) {
        return Err(binrw::Error::Io(write_err));
    }

    if let Err(write_err) = writer.write(&temp_buffer) {
        return Err(binrw::Error::Io(write_err));
    }

    Ok(())
}

#[binwrite]
#[derive(Debug)]
#[br(import(additional_data_length: u16))]
pub enum RequestDataOpt<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    Raw(#[br(count = additional_data_length)] Vec<u8>),
    Parsed(T),
}

impl<T> BinRead for RequestDataOpt<T>
where
    T: for<'a> binrw::BinWrite<Args<'a> = ()> + for<'a> binrw::BinRead<Args<'a> = ()>,
{
    type Args<'a> = (u16,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        // ALWAYS read a RequestDataOpt as a Raw Type
        let (len,) = args;
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf)?;
        Ok(RequestDataOpt::Raw(buf))
    }
}


impl<T> PartialEq for RequestDataOpt<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()> + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use RequestDataOpt::*;
        match (self, other) {
            (Raw(a), Raw(b)) => a == b,
            (Parsed(a), Parsed(b)) => a == b,
            (Raw(raw), Parsed(parsed)) | (Parsed(parsed), Raw(raw)) => {
                let mut buf = Vec::new();
                let mut writer = std::io::Cursor::new(&mut buf);

                // Use write_options with correct arguments
                parsed.write_options(&mut writer, binrw::Endian::Little, ()).ok();

                buf == *raw
            }
        }
    }
}

impl<T> Eq for RequestDataOpt<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()> + Eq,
{}

const BYTES_IN_A_WORD: u16 = 2;
const SIZE_OF_CIP_USINT: usize = mem::size_of::<CipUsint>();

#[binrw]
#[derive(Debug, PartialEq)]
#[brw(little)]
#[br(import(additional_data_length: u16))]
pub struct RequestData<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub total_word_size: CipUsint,
    // override the total_word_size by seeking back before it
    #[bw(seek_before = SeekFrom::Current(-1 * (mem::size_of::<CipUsint>() as i64)), write_with = write_cip_path_with_size)]
    #[br(args(total_word_size))]
    pub cip_path: CipPath,

    // Subtract the number of bytes taken up by the `total_word_size` field and the `cip_path``
    #[br(args(additional_data_length - (SIZE_OF_CIP_USINT as u16) - (BYTES_IN_A_WORD * total_word_size as u16)))]
    pub additional_data: RequestDataOpt<T>
}

// ======= Start of RequestData impl ========

impl<T> RequestData<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub fn new(word_size: Option<CipUsint>, path: CipPath, request_data_content: Option<T>) -> Self {
        RequestData {
            total_word_size: word_size.unwrap_or(0),
            cip_path: path,
            additional_data: match request_data_content {
                None => RequestDataOpt::Raw(vec![]),
                Some(content) => RequestDataOpt::Parsed(content),
            }
        }
    }
}


const SIZE_OF_SERVICE_CONTAINER: usize = mem::size_of::<ServiceContainer>();

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
#[br(import(additional_data_length: u16))]
pub struct MessageRouterRequest<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub service_container: ServiceContainer,

    // Subtract number of bytes in the service container 
    #[br(args(additional_data_length - SIZE_OF_SERVICE_CONTAINER as u16))]
    pub request_data: RequestData<T>,
}

// ======= Start of MessageRouterRequest impl ========

impl MessageRouterRequest<u8> {
    pub fn new(service_code: ServiceCode, path: CipPath) -> Self {
        Self::new_data(service_code, path, Option::<u8>::None)
    }
}

impl<T> MessageRouterRequest<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub fn new_data(
        service_code: ServiceCode,
        path: CipPath,
        request_data_content: Option<T>,
    ) -> Self {
        MessageRouterRequest {
            service_container: ServiceContainer::new(service_code, false),
            request_data: RequestData::new(None, path, request_data_content),
        }
    }
}

// ^^^^^^^^ End of MessageRouterRequest impl ^^^^^^^^
