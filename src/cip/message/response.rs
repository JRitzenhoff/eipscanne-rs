use binrw::{
    binrw,
    BinRead,
    BinWrite, // BinRead,  // trait for reading
};

use crate::cip::{message::data::{CipDataOpt}, types::CipUsint};

use super::shared::ServiceContainer;

#[derive(BinRead, BinWrite)]
#[brw(little, repr = CipUsint)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ResponseStatusCode {
    Success = 0x0000,
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
pub struct ResponseData
{
    #[brw(pad_before = 1)]
    pub status: ResponseStatusCode,
    pub additional_status_size: u8,

    #[br(try)]
    pub data: Option<CipDataOpt>,
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
pub struct MessageRouterResponse
{
    #[br(assert(service_container.response()))]
    pub service_container: ServiceContainer,
    pub response_data: ResponseData,
}
