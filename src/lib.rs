#![no_std]
#![no_main]

extern crate libc;

pub mod change_dir;
pub mod config;
pub mod mount;
pub mod string;
pub mod switch_root;
pub mod vector;

pub const SYSROOT: &str = "/sysroot";
pub const PUT_OLD: &str = "/sysroot/mnt";

pub const INIT: &str = "/usr/lib/systemd/systemd";
