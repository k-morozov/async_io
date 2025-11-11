pub fn run_server(port: u16) -> i32 {
    let sfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    if sfd == -1 {
        // error
    }
    //  unsafe {
    //     let flags = libc::fcntl(sfd, libc::F_GETFL, 0);
    //     let r = libc::fcntl(sfd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    //     dbg!(r);
    // };

    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: port.to_be(), // network byte order
        sin_addr: libc::in_addr {
            s_addr: libc::INADDR_ANY.to_be(),
        },
        sin_zero: [0; 8],
    };

    let addr_len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;

    unsafe {
        libc::bind(
            sfd,
            &addr as *const libc::sockaddr_in as *mut libc::sockaddr,
            addr_len,
        );
    }
    //error
    unsafe {
        libc::listen(sfd, 2);
    }

    sfd
}

pub fn handle_connection(sfd: i32) -> i32 {
    let mut peer_addr = libc::sockaddr {
        sa_data: [0; 14],
        sa_family: libc::AF_INET as u16,
    };
    let mut slen = 0;

    println!("prepare to call accept, sfd={sfd}");
    let cfd = unsafe { libc::accept(sfd, &mut peer_addr, &mut slen) };
    println!("accept was called, cfd={cfd}");

    unsafe {
        let flags = libc::fcntl(cfd, libc::F_GETFL, 0);
        libc::fcntl(cfd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    };
    cfd
}
