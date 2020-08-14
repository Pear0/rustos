use downcast_rs::DowncastSync;

pub trait DeviceDriver : DowncastSync {

}

impl_downcast!(sync DeviceDriver);


