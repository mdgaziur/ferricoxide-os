use core::sync::atomic::{AtomicBool, Ordering};
use spin::{Mutex, Once};
use crate::display::framebuffer::Framebuffer;
use crate::display::text_renderer::TextRenderer;

pub mod framebuffer;
mod text_renderer;

pub static FRAMEBUFFER: Once<Mutex<Framebuffer>> = Once::new();
pub static TEXT_RENDERER: Once<Mutex<TextRenderer>> = Once::new();
pub static USE_TEXT_RENDERER: AtomicBool = AtomicBool::new(true);

/// Initializes everything necessary to show stuff in the display.
/// 
/// # Safety:
/// Check [`Framebuffer::new`]
pub unsafe fn init() {
    init_framebuffer();
    init_text_renderer();
}

pub fn display_text(text: &str) {
    if USE_TEXT_RENDERER.load(Ordering::Relaxed) {
        TEXT_RENDERER.get().unwrap().lock().display_str(text);
        FRAMEBUFFER.get().unwrap().lock().flush();
    }
}

pub fn init_text_renderer() {
    let fb = FRAMEBUFFER.get().unwrap().lock();
    TEXT_RENDERER.call_once(|| {
        Mutex::new(TextRenderer::new(
            fb.width as usize,
            fb.height as usize,
        ))
    });
}

pub fn init_framebuffer() {
    FRAMEBUFFER.call_once(|| {
        Mutex::new(unsafe { Framebuffer::new() })
    });
}