use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn register() -> Result<Interrupted, ctrlc::Error> {
    let interrupted = Arc::new(AtomicBool::new(false));
    let b = interrupted.clone();
    ctrlc::set_handler(move || {
        let already_interrupted = b.load(Ordering::SeqCst);
        if already_interrupted {
            println!("User requested abort (Ctrl+C twice)");
            std::process::exit(1);
        }
        println!("Waiting for in-flight requests (Ctrl+C again to abort)...");
        b.store(true, Ordering::SeqCst);
    })?;
    Ok(Interrupted { interrupted })
}

#[derive(Clone)]
pub struct Interrupted {
    interrupted: Arc<AtomicBool>,
}

impl Interrupted {
    pub fn interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }
}
