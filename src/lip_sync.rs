use godot::prelude::*;
use lazy_static::lazy_static;
use rand::{rngs::ThreadRng, Rng};
use std::{
    collections::{HashMap, VecDeque},
    ops::{Add, Div, Index, Mul, MulAssign},
    sync::mpsc,
    sync::{Arc, Mutex},
    thread,
};

use crate::{job, job::JobMessage};

const LIP_SYNC_UPDATED: &str = "updated";
const LIP_SYNC_PANICKED: &str = "panicked";

#[derive(GodotClass)]
#[class(base = Node)]
pub struct LipSyncRs {
    join_handle: Option<thread::JoinHandle<()>>,
    sender: mpsc::Sender<job::JobMessage>,
    receiver: mpsc::Receiver<job::JobMessage>,
    #[base]
    base: Base<Node>,
}

unsafe impl Sync for LipSyncRs {}

unsafe impl Send for LipSyncRs {}

#[godot_api]
impl LipSyncRs {
    #[signal]
    fn updated();

    #[signal]
    fn panicked();

    #[func]
    pub fn update(&mut self, stream: Array<f32>) {
        self.sender
            .send(JobMessage::InputData(stream))
            .expect("Unable to send stream to thread");
    }

    #[func]
    pub fn poll(&mut self) {
        match self.receiver.try_recv() {
            Ok(v) => match v {
                JobMessage::OutputData(od) => {
                    // godot_print!("Emitted signal: {:?}", LIP_SYNC_UPDATED);

                    self.base.emit_signal(
                        LIP_SYNC_UPDATED.into(),
                        &[Variant::from(Dictionary::from(od))],
                    );
                }
                _ => {
                    // Unexpected data
                    self.sender.send(JobMessage::Shutdown).expect("When shutting down thread because of invalid message, encoutered error. Shutting down anyways.");
                }
            },
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected {
                    // godot_print!("Emitted signal: {:?}", LIP_SYNC_PANICKED);

                    self.base
                        .emit_signal(LIP_SYNC_PANICKED.into(), &[Variant::from(format!("{}", e))]);
                }
            }
        }
    }

    #[func]
    pub fn shutdown(&mut self) {
        self.sender.send(JobMessage::Shutdown).expect("When shutting down thread because of invalid message, encountered error. Shutting down anyways.");
        self.join_handle
            .take()
            .expect("Unable to take join_handle")
            .join()
            .expect("Unable to join thread");
    }
}

#[godot_api]
impl INode for LipSyncRs {
    fn init(base: Base<Self::Base>) -> Self {
        let (jh, s, r) = job::create_job().expect("Unable to create job thread");

        LipSyncRs {
            join_handle: Some(jh),
            sender: s,
            receiver: r,
            base,
        }
    }
}
