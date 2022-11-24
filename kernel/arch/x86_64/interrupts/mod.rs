mod gdt;

use gdt::Gdt;
use lazy_static::lazy_static;

use crate::arch::cpu::CPU;
use crate::arch::mm::paging::entry::EntryFlags;
use crate::arch::x86_64::mm::MemoryController;
use pic8259::ChainedPics;
use spin::{Mutex, Once};
use x86_64::instructions::hlt;
use x86_64::instructions::port::Port;
use x86_64::instructions::segmentation::Segment;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::CS;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<Gdt> = Once::new();

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        idt.page_fault.set_handler_fn(page_fault_handler);

        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX as u16);
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);

        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self as u8)
    }
}

static DOUBLE_FAULT_IST_INDEX: usize = 0;

pub fn init_interrupts(memory_controller: &mut MemoryController) {
    let double_fault_stack = memory_controller
        .alloc_stack(20, EntryFlags::empty())
        .expect("could not allocate stack for double fault stack");
    info!("Initialized double fault stack");

    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX] =
            VirtAddr::new(double_fault_stack.top() as u64);
        tss
    });
    info!("Created tss with double fault stack");

    let mut code_selector = SegmentSelector(0);
    let mut tss_selector = SegmentSelector(0);

    let gdt = GDT.call_once(|| {
        let mut gdt = Gdt::new();
        code_selector = gdt.add_entry(Gdt::kernel_code_segment());
        tss_selector = gdt.add_entry(Gdt::tss_segment(&tss));

        gdt
    });
    info!("Initialized gdt");

    gdt.load();
    info!("Loaded gdt");

    unsafe {
        CS::set_reg(code_selector);
        info!("Set CS successful");

        load_tss(tss_selector);
        info!("Loaded TSS successfully");
    }

    IDT.load();
    info!("Loaded IDT");

    unsafe { PICS.lock().initialize() }
    info!("Initialized PIC");

    CPU::enable_interrupts();
    info!("Enabled interrupts");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    error!("Exception: Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    error!("Exception: Double fault\n{:#?}", stack_frame);

    // TODO: The following doesn't get executed for some reason sometimes
    info!("Error code: {}", error_code);

    info!("Halting CPU!");
    hlt();

    // to tell rust that this is diverging function
    loop {}
}

extern "x86-interrupt" fn timer_interrupt_handler(_: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    error!(
        "Exception: Page fault\n{:#?}\nError code: {:?}",
        stack_frame, error_code
    );
    // TODO: handle page fault from userland applications(in future)
    CPU::halt();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_: InterruptStackFrame) {
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
