use gdnative::{
    api::{AudioEffectRecord, AudioServer, AudioStreamSample},
    prelude::*,
};
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

const LIP_SYNC_UPDATED: &str = "lip_sync_updated";
const LIP_SYNC_PANICKED: &str = "lip_sync_panicked";

#[derive(NativeClass)]
#[inherit(Reference)]
#[user_data(user_data::RwLockData<LipSync>)]
#[register_with(Self::register_lip_sync)]
pub struct LipSync {
    join_handle: Option<thread::JoinHandle<()>>,
    sender: mpsc::Sender<job::JobMessage>,
    receiver: mpsc::Receiver<job::JobMessage>,
}

unsafe impl Sync for LipSync {}
unsafe impl Send for LipSync {}

#[methods]
impl LipSync {
    fn new(_owner: &Reference) -> Self {
        let (jh, s, r) = job::create_job().expect("Unable to create job thread");

        LipSync {
            join_handle: Some(jh),
            sender: s,
            receiver: r,
        }
    }

    fn register_lip_sync(builder: &ClassBuilder<Self>) {
        builder.add_signal(Signal {
            name: &LIP_SYNC_UPDATED,
            args: &[SignalArgument {
                name: "result",
                default: Variant::from_dictionary(&Dictionary::default()),
                export_info: ExportInfo::new(VariantType::Dictionary),
                usage: PropertyUsage::DEFAULT,
            }],
        });

        builder.add_signal(Signal {
            name: &LIP_SYNC_PANICKED,
            args: &[SignalArgument {
                name: "message",
                default: Variant::from_str("invalid error"),
                export_info: ExportInfo::new(VariantType::GodotString),
                usage: PropertyUsage::DEFAULT,
            }],
        });
    }

    #[export]
    pub fn update(&mut self, _owner: &Reference, stream: TypedArray<f32>) {
        self.sender
            .send(JobMessage::InputData(stream))
            .expect("Unable to send stream to thread");
    }

    #[export]
    pub fn poll(&self, owner: &Reference) {
        match self.receiver.try_recv() {
            Ok(v) => match v {
                JobMessage::OutputData(od) => {
                    owner.emit_signal(
                        LIP_SYNC_UPDATED,
                        &[Variant::from_dictionary(&Dictionary::from(od))],
                    );
                }
                _ => {
                    // Unexpected data
                    self.sender.send(JobMessage::Shutdown).expect("When shutting down thread because of invalid message, encoutered error. Shutting down anyways.");
                }
            },
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected {
                    owner.emit_signal(LIP_SYNC_PANICKED, &[Variant::from_str(format!("{}", e))]);
                }
            }
        }
    }

    #[export]
    pub fn shutdown(&mut self, _owner: &Reference) {
        self.sender.send(JobMessage::Shutdown).expect("When shutting down thread because of invalid message, encoutered error. Shutting down anyways.");
        self.join_handle
            .take()
            .expect("Unable to take join_handle")
            .join()
            .expect("Unable to join thread");
    }
}
