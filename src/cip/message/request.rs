use std::io::SeekFrom;
use std::mem;

use binrw::{
    binrw, BinRead, BinResult, BinWrite
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

#[binrw]
#[derive(Debug, PartialEq)]
#[brw(little)]
pub struct RequestData<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub total_word_size: CipUsint,
    // override the total_word_size by seeking back before it
    #[bw(seek_before = SeekFrom::Current(-1 * (mem::size_of::<CipUsint>() as i64)), write_with = write_cip_path_with_size)]
    pub cip_path: CipPath,

    // Always deserialize into raw bytes
    #[br(try, count = total_word_size)]
    raw_additional_data: Option<Vec<u8>>,

    // Parsed view (not written/read directly by binrw)
    #[brw(ignore)]
    additional_data: Option<T>,
}

// ======= Start of RequestData impl ========

impl<T> RequestData<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub fn additional_data(&mut self) -> binrw::BinResult<&T> {
        if self.additional_data.is_none() && self.raw_additional_data.is_none() {
            return Err(binrw::Error::AssertFail {
                pos: 0,
                message: "Unable to read additional data, underlying raw_additional_data is None".to_string(),
            });
        }

        if self.additional_data.is_none() {
            let mut cursor = std::io::Cursor::new(self.raw_additional_data.as_ref().unwrap());
            let parsed = T::read_le(&mut cursor)?;
            self.additional_data = Some(parsed);
        }

        Ok(self.additional_data.as_ref().unwrap())
    }

    pub fn raw_additional_data(&self) -> Result<&[u8], binrw::Error> {
        if self.raw_additional_data.is_none() {
            return Err(binrw::Error::AssertFail {
                pos: 0,
                message: "Unable to read, raw_additional_data is None".to_string(),
            });
        }
        
        Ok(self.raw_additional_data.as_ref().unwrap())
    }
}

impl<T> RequestData<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub fn new(word_size: Option<CipUsint>, path: CipPath, request_data_content: Option<T>) -> Self {
        RequestData {
            total_word_size: word_size.unwrap_or(0),
            cip_path: path,
            raw_additional_data: None,
            additional_data: request_data_content,
        }
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
pub struct MessageRouterRequest<T>
where
    T: for<'a> BinWrite<Args<'a> = ()> + for<'a> BinRead<Args<'a> = ()>,
{
    pub service_container: ServiceContainer,
    pub request_data: RequestData<T>,
}

// ======= Start of MessageRouterRequest impl ========

impl MessageRouterRequest<u8> {
    pub fn new(service_code: ServiceCode, path: CipPath) -> Self {
        Self::new_data(service_code, path, None)
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
