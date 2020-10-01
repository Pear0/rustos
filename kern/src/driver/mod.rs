use downcast_rs::DowncastSync;

pub mod net;

pub trait DeviceDriver : DowncastSync {

}

impl_downcast!(sync DeviceDriver);


