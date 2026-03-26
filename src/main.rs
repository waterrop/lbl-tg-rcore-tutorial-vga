#![cfg_attr(target_arch = "riscv64", no_std)]
#![cfg_attr(target_arch = "riscv64", no_main)]

#[cfg(target_arch = "riscv64")]
use core::arch::global_asm;

#[cfg(target_arch = "riscv64")]
use lbl_tg_rcore_tutorial_vga::{clear_screen, draw_pixel, init, resolution};
#[cfg(target_arch = "riscv64")]
use tg_sbi::{console_putchar, shutdown};

#[cfg(target_arch = "riscv64")]
global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top
    call rust_main

    .section .bss.uninit
    .align 12
boot_stack:
    .space 16384
boot_stack_top:
"#
);

#[cfg(target_arch = "riscv64")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    print_line("boot: panic");
    shutdown(true);
}

#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    print_line("boot: start vga demo");

    match init() {
        Ok(()) => match resolution() {
            Ok((width, height)) => {
                let _ = clear_screen(0x0000_0000);
                draw_test_pattern(width, height);
                print_line("boot: init ok");
                print_line("boot: close qemu window or ctrl+c to exit");
                idle_forever();
            }
            Err(_) => {
                print_line("boot: resolution failed");
            }
        },
        Err(_) => {
            print_line("boot: init failed");
        }
    }

    shutdown(false);
}

#[cfg(target_arch = "riscv64")]
fn draw_test_pattern(width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let max_x = width - 1;
    let max_y = height - 1;
    let center_x = width / 2;
    let center_y = height / 2;

    for y in 0..height {
        for x in 0..width {
            let red = ((x * 255) / width) & 0xff;
            let green = ((y * 255) / height) & 0xff;
            let blue = (((x + y) * 255) / (width + height)) & 0xff;
            let color = (red << 16) | (green << 8) | blue;
            let _ = draw_pixel(x, y, color);
        }
    }

    for x in 0..width {
        let _ = draw_pixel(x, center_y, 0x00ff_ffff);
    }
    for y in 0..height {
        let _ = draw_pixel(center_x, y, 0x00ff_ffff);
    }

    let _ = draw_pixel(0, 0, 0x00ff_0000);
    let _ = draw_pixel(max_x, 0, 0x0000_ff00);
    let _ = draw_pixel(0, max_y, 0x0000_00ff);
    let _ = draw_pixel(max_x, max_y, 0x00ff_ff00);
    let _ = draw_pixel(center_x, center_y, 0x00ff_ffff);
}

#[cfg(target_arch = "riscv64")]
fn idle_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "riscv64")]
fn print_line(message: &str) {
    for byte in message.bytes() {
        console_putchar(byte);
    }
    console_putchar(b'\n');
}

#[cfg(not(target_arch = "riscv64"))]
fn main() {}
