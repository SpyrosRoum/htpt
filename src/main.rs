use std::{
    ffi::{CStr, CString},
    mem::{size_of, MaybeUninit},
    process::exit,
};

use libc::syscall;

const PORT: u16 = 6971;

/// Accept only local connections
const INADDR_LOCAL: u32 = 0x0100007F;

fn report_c_error() {
    unsafe {
        let errno_ptr = libc::__errno_location();
        let ptr = libc::strerror(*errno_ptr);
        let err_msg = CStr::from_ptr(ptr);
        eprintln!("{:?}", err_msg);
    }
}

const fn htons(n: u16) -> u16 {
    let first = (n & 255) << 8;
    let second = (n >> 8) & 255;
    first | second
}

fn write_all_to_fd(fd: libc::c_int, msg: &str) -> libc::ssize_t {
    unsafe {
        let buff = CString::new(msg).unwrap();
        let buff = buff.as_bytes();

        let mut c = 0;
        while c < msg.len() {
            let to_send = &buff[c..];
            let wrote = libc::write(fd, to_send.as_ptr() as _, to_send.len());
            if wrote < 0 {
                return wrote;
            }
            c += wrote as usize;
        }

        0
    }
}

fn main() {
    unsafe {
        let fd = syscall(libc::SYS_socket, libc::AF_INET, libc::SOCK_STREAM, 0);
        if fd < 0 {
            eprint!("Could not create TCP socket: ");
            report_c_error();
            exit(1);
        }

        let sock_addr = libc::sockaddr_in {
            sin_family: libc::AF_INET as libc::sa_family_t,
            sin_port: htons(PORT) as libc::in_port_t,
            sin_addr: libc::in_addr {
                s_addr: INADDR_LOCAL,
            },
            sin_zero: [0; 8],
        };
        let r = syscall(
            libc::SYS_bind,
            fd,
            &sock_addr as *const _,
            size_of::<libc::sockaddr>(),
        );
        if r < 0 {
            eprint!("Failed to bind to address: ");
            report_c_error();
            exit(1);
        }

        let r = syscall(libc::SYS_listen, fd, 5);
        if r < 0 {
            eprint!("Could not start listening: ");
            report_c_error();
            exit(1);
        }

        println!("Listening on port: {}", PORT);

        let e = loop {
            let cli_len = size_of::<libc::socklen_t>();
            let client: MaybeUninit<libc::sockaddr_in> = MaybeUninit::uninit();
            let conn_fd =
                syscall(libc::SYS_accept, fd, client.as_ptr(), &cli_len as *const _) as libc::c_int;
            if conn_fd < 0 {
                eprint!("Failed to accept connection: ");
                report_c_error();
                break 1;
            }
            let _client = client.assume_init();
            println!("Connected to client");

            // Standard Http 1.1 things
            write_all_to_fd(conn_fd, "HTTP/1.1 200 OK\r\n");
            write_all_to_fd(conn_fd, "Server: htpt\r\n");
            write_all_to_fd(conn_fd, "Content-Type: text/html\r\n");
            write_all_to_fd(conn_fd, "Connection: Closed\r\n");
            write_all_to_fd(conn_fd, "\r\n");

            write_all_to_fd(conn_fd, "<h1>Hello World!</h1>\n");
            write_all_to_fd(conn_fd, "\r\n");

            syscall(libc::SYS_close, conn_fd);
        };

        syscall(libc::SYS_close, fd);
        exit(e);
    }
}
