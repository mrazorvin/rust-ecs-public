use std::net::TcpStream;

pub static mut DEVTOOLS: Option<TcpStream> = None;

pub fn init() -> Result<(), std::io::Error> {
    let devtools = std::net::TcpStream::connect("127.0.0.1:34254")?;

    unsafe { DEVTOOLS = Some(devtools) };

    Ok(())
}
