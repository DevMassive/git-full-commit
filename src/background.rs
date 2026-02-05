use crate::git::{self, FileDiff};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

pub enum Request {
    GetCommitDiff(PathBuf, String),
}

pub enum Response {
    CommitDiff(String, Vec<FileDiff>),
}

pub struct BackgroundWorker {
    tx: Sender<Request>,
    rx: Receiver<Response>,
}

impl Default for BackgroundWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundWorker {
    pub fn new() -> Self {
        let (req_tx, req_rx) = channel::<Request>();
        let (res_tx, res_rx) = channel::<Response>();

        thread::spawn(move || {
            while let Ok(request) = req_rx.recv() {
                match request {
                    Request::GetCommitDiff(repo_path, hash) => {
                        let diff = git::get_commit_diff(&repo_path, &hash).unwrap_or_default();
                        let _ = res_tx.send(Response::CommitDiff(hash, diff));
                    }
                }
            }
        });

        Self {
            tx: req_tx,
            rx: res_rx,
        }
    }

    pub fn request_commit_diff(&self, repo_path: PathBuf, hash: String) {
        let _ = self.tx.send(Request::GetCommitDiff(repo_path, hash));
    }

    pub fn poll(&self) -> Option<Response> {
        self.rx.try_recv().ok()
    }
}
