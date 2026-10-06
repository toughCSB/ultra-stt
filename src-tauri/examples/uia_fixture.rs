use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    output.write_all(b"UIA3")?;
    output.flush()?;
    loop {
        let mut request = [0; 13];
        if input.read_exact(&mut request).is_err() {
            return Ok(());
        }
        let process_id = u32::from_le_bytes(request[1..5].try_into().unwrap());
        if request[0] == 1 {
            match process_id {
                1 => loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                },
                3 => output.write_all(&65_537u32.to_le_bytes())?,
                4 => {
                    output.write_all(&2u32.to_le_bytes())?;
                    output.write_all(&[0xff, 0xff])?;
                }
                0 => output.write_all(&0u32.to_le_bytes())?,
                5 => output.write_all(&u32::MAX.to_le_bytes())?,
                _ => {
                    let text = "선택된 글 / terminal selection".as_bytes();
                    output.write_all(&(text.len() as u32).to_le_bytes())?;
                    output.write_all(text)?;
                }
            }
            output.flush()?;
            continue;
        }
        match process_id {
            0 => output.write_all(&[0])?,
            1 => loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            },
            2 => std::process::exit(1),
            3 => output.write_all(&[65])?,
            4 => {
                output.write_all(&[2, 0])?;
                output.flush()?;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
            }
            _ => {
                output.write_all(&[2])?;
                output.write_all(&(std::process::id() as i32).to_le_bytes())?;
                output.write_all(&(process_id as i32).to_le_bytes())?;
            }
        }
        output.flush()?;
    }
}
