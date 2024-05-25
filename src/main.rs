mod color;

use std::io::Write;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::{Context, Result};
use color::Color;
use color::HslColor;
use crossterm::{cursor, queue, style, terminal};
use rand::{Rng, RngCore};
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;

#[derive(Clone)]
struct Glyph {
    character: char,
    color: Color,
}

impl Glyph {
    fn new(character: char, color: Color) -> Self {
        Self { character, color }
    }

    fn new_random<R: Rng>(rand: &mut R, color: Color) -> Self {
        let characters = "ﾊﾐﾋｰｳｼﾅﾓﾆｻﾜﾂｵﾘｱﾎﾃﾏｹﾒｴｶｷﾑﾕﾗｾﾈｽﾀﾇﾍｦｲｸｺｿﾁﾄﾉﾌﾔﾖﾙﾚﾛﾝ012345789Z:.\"=*+-<>¦╌ç";
        Self {
            // @TODO: Don't use chars iterator to count chars here every time.
            character: characters
                .chars()
                .nth(rand.gen_range(0..characters.chars().count()))
                .unwrap(),
            color,
        }
    }

    fn empty() -> Self {
        Self {
            character: ' ',
            color: Color::from_rgb(0, 0, 0),
        }
    }

    fn render<W: Write>(&self, out: &mut W) -> Result<()> {
        queue!(
            out,
            style::SetForegroundColor(style::Color::Rgb {
                r: self.color.r,
                g: self.color.g,
                b: self.color.b
            })
        )?;
        queue!(out, style::Print(self.character.to_string())).context("write glyph to output")?;
        Ok(())
    }

    fn fade_color(&mut self) {
        let hsl = self.color.as_hsl();
        let new_color = HslColor::new(hsl.h, hsl.s * 0.90, hsl.l * 0.90);
        if new_color.s < 10.0 || new_color.l < 10.0 {
            self.color = HslColor::new(hsl.h, 10.0, 10.0).into();
        } else {
            self.color = new_color.into();
        }
    }
}

#[derive(Clone)]
struct Column {
    height: u16,
    base_color: Color,
    glyphs: Vec<Glyph>,
    active_index: usize,
}

impl Column {
    fn new(height: u16, base_color: Color) -> Self {
        Self {
            height,
            base_color,
            glyphs: vec![Glyph::empty(); height as usize],
            active_index: 0,
        }
    }

    fn render<W: Write>(&self, out: &mut W, y: u16) -> Result<()> {
        self.glyphs[y as usize].render(out)?;
        Ok(())
    }

    fn step<R: Rng>(&mut self, rand: &mut R) {
        for glyph in &mut self.glyphs {
            glyph.fade_color();
        }

        if self.active_index == 0 && rand.gen::<f32>() > 0.1 {
            return;
        }

        self.glyphs[self.active_index] = Glyph::new_random(rand, self.base_color);
        self.active_index += 1;

        if self.active_index >= self.height as usize {
            self.active_index = 0;
        }
    }
}

struct MatrixWaterfall {
    width: u16,
    height: u16,
    base_color: Color,
    columns: Vec<Column>,
}

impl MatrixWaterfall {
    fn new(width: u16, height: u16, base_color: Color) -> Self {
        Self {
            width,
            height,
            base_color,
            columns: vec![Column::new(height, base_color); width as usize],
        }
    }

    fn render<W: Write>(&self, out: &mut W) -> Result<()> {
        queue!(out, cursor::Hide)?;
        queue!(out, cursor::MoveTo(0, 0))?;
        queue!(
            out,
            style::SetBackgroundColor(style::Color::Rgb { r: 0, g: 0, b: 0 })
        )?;

        for y in 0..self.height {
            for column in &self.columns {
                column.render(out, y)?;
            }
        }
        queue!(out, style::ResetColor)?;
        queue!(out, cursor::Show)?;
        out.flush().context("flush output")?;
        Ok(())
    }

    fn step<R: RngCore>(&mut self, rand: &mut R) {
        for column in &mut self.columns {
            column.step(rand);
        }
    }
}

fn usage(command: &str) {
    eprintln!("Usage:");
    eprintln!("  {command} [OPTIONS]");
    eprintln!("Options:");
    eprintln!("  --color HEXCOLOR");
    eprintln!("  --speed UPDATES_PER_SEC");
}

fn main() -> Result<()> {
    let mut args = std::env::args();
    let command = args.next().expect("args should have at least command");

    let mut base_color = Color::from_rgb(0, 255, 43);
    let mut speed: u32 = 13;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" => {
                usage(&command);
                std::process::exit(0);
            }
            "--color" => {
                let hexcolor = args.next().expect("hex color provided after --color");
                base_color = Color::from_hexstring(hexcolor.as_str())
                    .expect("provided color to be valid hexstring");
            }
            "--speed" => {
                speed = args
                    .next()
                    .expect("speed provided after --speed")
                    .parse::<u32>()
                    .expect("specified speed is a number");
                if speed > 120 {
                    eprintln!("Speed is limited to 0-120");
                    usage(&command);
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument {arg}");
                usage(&command);
                std::process::exit(1);
            }
        }
    }

    let (width, height) = terminal::size().context("determine terminal size")?;

    let mut waterfall = MatrixWaterfall::new(width, height, base_color);
    let mut stdout = std::io::stdout();

    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time to have passed since UNIX_EPOCH")
        .as_micros() as u64;
    let mut rand = Xoshiro256PlusPlus::seed_from_u64(seed);

    let frame_wait = (1000f64 / speed as f64).round() as u64;

    loop {
        waterfall.render(&mut stdout)?;
        waterfall.step(&mut rand);
        std::thread::sleep(Duration::from_millis(frame_wait));
    }
}
