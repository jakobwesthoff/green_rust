mod color;

use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::*;

use color::{Color, HslColor};
use rand::{Rng, RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

struct MatrixWaterfall {
    width: u16,
    height: u16,
    base_color: Color,
    columns: Vec<Column>,
}

impl MatrixWaterfall {
    fn new(width: u16, height: u16, base_color: Color) -> Self {
        let mut columns = vec![];
        for x in 0..width {
            columns.push(Column::new(height, base_color));
        }

        Self {
            width,
            height,
            base_color,
            columns,
        }
    }

    fn render<W: Write>(&self, out: &mut W) -> Result<()> {
        out.queue(cursor::MoveTo(0, 0))?;
        out.queue(style::SetBackgroundColor(style::Color::Rgb {
            r: 0,
            b: 0,
            g: 0,
        }))?;

        for y in 0..self.height {
            for column in &self.columns {
                column.render(out, y)?;
            }
        }
        out.queue(style::ResetColor)?;

        out.flush()?;
        Ok(())
    }

    fn step<R: RngCore>(&mut self, rand: &mut R) {
        for i in 0..self.columns.len() {
            self.columns[i].step(rand);
        }
    }
}

struct Column {
    height: u16,
    glyphs: Vec<Glyph>,
    active_index: usize,
    base_color: Color,
}

impl Column {
    const CHAR_POOL: [&'static str; 5] = ["a", "b", "c", "d", "d"];

    fn new(height: u16, base_color: Color) -> Self {
        let mut glyphs = vec![];
        for _ in 0..height {
            glyphs.push(Glyph::new(" ".to_string(), base_color));
        }

        Self {
            height,
            glyphs,
            active_index: 0,
            base_color,
        }
    }

    fn render<W: Write>(&self, out: &mut W, y: u16) -> Result<()> {
        self.glyphs[y as usize].render(out)?;
        Ok(())
    }

    fn step<R: Rng>(&mut self, rand: &mut R) {
        if self.active_index == 0 {
            // Wait random amount before starting the column again
            if rand.gen::<f32>() > 0.1 {
                return;
            }
        }
        if self.active_index == (self.height - 1) as usize {
            self.active_index = 0;
        } else {
            self.active_index += 1;
        }

        // Update color fade
        for i in 0..self.glyphs.len() {
            self.glyphs[i].fade_color();
        }

        let char_index = rand.gen_range(0..Self::CHAR_POOL.len());
        self.glyphs[self.active_index] =
            Glyph::new(Self::CHAR_POOL[char_index].to_string(), self.base_color);
    }
}

struct Glyph {
    character: String,
    color: Color,
}

impl Glyph {
    fn new(character: String, color: Color) -> Self {
        Self { character, color }
    }

    fn render<W: Write>(&self, out: &mut W) -> Result<()> {
        out.queue(style::SetForegroundColor(style::Color::Rgb {
            r: self.color.r,
            g: self.color.g,
            b: self.color.b,
        }))?;
        out.queue(style::Print(&self.character))?;
        Ok(())
    }

    fn fade_color(&mut self) {
        let color = self.color.as_hsl();
        let new_color = Color::from(HslColor::new(color.h, color.s * 0.9, color.l * 0.9));
        self.color = new_color;
    }
}

fn main() -> Result<()> {
    let (width, height) = terminal::size().expect("to be able to retrieve terminal size");
    let mut stdout = std::io::stdout();

    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("If time since UNIX_EPOCH is 0 there is something wrong?")
        .as_micros();
    let mut rand = Xoshiro256PlusPlus::seed_from_u64(micros as u64);

    let mut waterfall = MatrixWaterfall::new(width, height, Color::from_rgb(3, 160, 98));
    loop {
        waterfall.render(&mut stdout)?;
        waterfall.step(&mut rand);
        std::thread::sleep(Duration::from_millis(50));
    }
}
