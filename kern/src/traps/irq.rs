use pi::interrupt::Interrupt;

use crate::process::{TICK, State};
use crate::traps::TrapFrame;
use crate::SCHEDULER;

pub fn handle_irq(interrupt: Interrupt, tf: &mut TrapFrame) {
    unimplemented!("handle_irq()")
}
