use crate::*;

pub fn vnic_state() -> OsResult<bool> {
    unsafe { do_hypercall1r!(HP_VNIC_STATE) }.map(|v| v != 0)
}

pub fn vnic_get_info() -> OsResult<u64> {
    unsafe { do_hypercall1r!(HP_VNIC_GET_INFO) }
}

pub fn vnic_send_frame(buf: &[u8]) -> OsResult<()> {
    assert!(buf.len() <= 1600); // maximum frame size
    unsafe { do_hypercall0r!(HP_VNIC_SEND, buf.as_ptr() as u64, buf.len() as u64) }
}

pub fn vnic_receive_frame(buf: &mut [u8]) -> OsResult<usize> {
    assert!(buf.len() >= 1600); // maximum frame size
    unsafe { do_hypercall1r!(HP_VNIC_RECEIVE, buf.as_ptr() as u64) }.map(|v| v as usize)
}
