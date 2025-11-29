pub fn run_server(port: u16) -> i32 {
    log::info!("Starting server: port={port}");

    let sfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    if sfd == -1 {
        // error
    }

    unsafe {
        let flags = libc::fcntl(sfd, libc::F_GETFL, 0);
        libc::fcntl(sfd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    };

    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: port.to_be(),
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

pub fn handle_connection(sockfd: i32) -> i32 {
    let mut peer_addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    let mut slen = std::mem::size_of::<libc::sockaddr_in>() as u32;

    log::info!("Prepare to call accept, sockfd={sockfd}");
    let cfd = unsafe {
        libc::accept(
            sockfd,
            &mut peer_addr as *mut libc::sockaddr_in as *mut libc::sockaddr,
            &mut slen,
        )
    };
    if -1 == cfd {
        return -1;
    }

    let ip_addr = std::net::Ipv4Addr::from(u32::from_be(peer_addr.sin_addr.s_addr));
    let port = u16::from_be(peer_addr.sin_port);

    log::info!("Client connected: cfd={cfd}, ip_addr={ip_addr}, port={port}");

    unsafe {
        let flags = libc::fcntl(cfd, libc::F_GETFL, 0);
        libc::fcntl(cfd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    };
    cfd
}
