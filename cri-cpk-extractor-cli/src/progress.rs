use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};
use cri_archive_lib::cpk::file::CpkFile;

#[derive(Debug)]
pub struct Progress {
    bar: ProgressBar,
    lock: AtomicBool
}

impl Progress {
    pub fn new(list: &[CpkFile]) -> Self {
        let bar = ProgressBar::new(list.len() as u64);
        let color_fmt = match Term::stdout().features().true_colors_supported() {
            true => "#DA70D6/#9932CC", false => "135/90"
        };
        let template_fmt = format!("[{{elapsed_precise}}] {{bar:40.{}}} {{pos:>7}}/{{len:7}} ({{percent_precise}}%) {{msg}}", color_fmt);
        let style = ProgressStyle::with_template(&template_fmt)
            .unwrap()
            .progress_chars("##-");
        bar.set_style(style);
        bar.tick();
        Self { bar, lock: AtomicBool::new(false) }
    }

    #[inline]
    fn acquire(&mut self) {
        while self.lock.swap(true, Ordering::Acquire) {}
    }

    #[inline]
    fn unacquire(&mut self) {
        self.lock.store(false, Ordering::Release);
    }

    #[inline]
    pub fn set_current_file(&self, file: &CpkFile) {
        unsafe { &mut *(&raw const *self as *mut Self) }.set_current_file_inner(file)
    }

    fn set_current_file_inner(&mut self, file: &CpkFile) {
        self.acquire();
        self.bar.set_message(format!("{}/{}", file.directory(), file.file_name()));
        self.unacquire();
    }

    #[inline]
    pub fn read_one(&self) {
        unsafe { &mut *(&raw const *self as *mut Self) }.read_one_inner()
    }

    fn read_one_inner(&mut self) {
        self.acquire();
        self.bar.set_position(self.bar.position() + 1);
        self.unacquire();
    }

    pub fn get_duration(&self) -> Duration {
        self.bar.duration()
    }
}

unsafe impl Send for Progress {}