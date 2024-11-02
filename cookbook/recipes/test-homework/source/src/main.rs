use std::env::args;
use std::fs::OpenOptions;
use std::io::{Read, Write};

use anyhow::{anyhow, bail, Context, Result};

const DRIVER_PATH: &str = "/scheme/homework";

fn main() -> Result<()> {
    let args: Vec<String> = args().collect();

    match args
        .get(1)
        .and_then(|a| Some(a.as_str()))
        .context("Missing read or write command")?
    {
        "read" => {
            //println!("cli: read");
            let mut handle = OpenOptions::new()
                .read(true)
                .open(DRIVER_PATH)
                .context("Failed to open driver")?;
            //println!("cli: opened file {:?}", handle);

            let mut buf: [u8; 4] = [0, 1, 0 ,1];
            handle
                .read(&mut buf)
                .context("Failed to read from driver")?;
            //println!("cli: read buffer {:?}", buf);
            println!("{}", u32::from_le_bytes(buf));
            Ok(())
        }
        "write" => {
            //println!("cli: write");
            let input: usize = args
                .get(2)
                .ok_or(anyhow!("Missing value to write"))?
                .parse()
                .context("Failed to parse input as unsigned integer")?;
            let mut handle = OpenOptions::new()
                .read(true)
                .write(true)
                .open(DRIVER_PATH)
                .context("Failed to open driver")?;
            //println!("cli: opened file {:?}", handle);
            handle
                .write(&input.to_ne_bytes())
                .context("Failed to write to driver")?;
            Ok(())
        }
        "slot" => {
            let input: u64 = args
                .get(2)
                .ok_or(anyhow!("Missing slot number"))?
                .parse()
                .context("Failed to parse input as unsigned integer")?;
            let mut handle = OpenOptions::new()
                .read(true)
                .write(true)
                .open(DRIVER_PATH)
                .context("Failed to open driver")?;
            handle.set_len(input)?;
            Ok(())
        }
        _ => bail!("Unrecognized command"),
    }
}
