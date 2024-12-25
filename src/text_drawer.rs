use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text, TextStyle, TextStyleBuilder},
    Drawable,
};
use ssd1306::{
    mode::BufferedGraphicsMode, prelude::WriteOnlyDataCommand, size::DisplaySize, Ssd1306,
};
use std::fmt;

pub struct TextDrawer<'a, DI, SIZE: DisplaySize> {
    display: Ssd1306<DI, SIZE, BufferedGraphicsMode<SIZE>>,
    default_char_style: MonoTextStyle<'a, BinaryColor>,
    default_text_style: TextStyle,
    bounds: Rectangle,
}

#[derive(Debug)]
pub enum TextError<E: std::fmt::Debug> {
    DrawError(E),
    DoesNotFit,
}

impl<E: std::fmt::Debug> fmt::Display for TextError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::DoesNotFit => write!(f, "Text does not fit within the display bounds"),
            TextError::DrawError(err) => write!(f, "Drawing error: {:?}", err),
        }
    }
}

impl<E: std::fmt::Debug + 'static> std::error::Error for TextError<E> {}

pub type DisplayType<DI, SIZE> = Ssd1306<DI, SIZE, BufferedGraphicsMode<SIZE>>;
pub type DisplayError<DI, SIZE> = <DisplayType<DI, SIZE> as DrawTarget>::Error;

impl<'a, DI, SIZE> TextDrawer<'a, DI, SIZE>
where
    DI: WriteOnlyDataCommand,
    SIZE: DisplaySize,
{
    pub fn new(display: DisplayType<DI, SIZE>, font: &'a MonoFont<'a>) -> Self {
        let default_char_style = MonoTextStyleBuilder::new()
            .font(font)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();
        let default_text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

        let bounds = display.bounding_box();

        Self {
            display,
            default_char_style,
            default_text_style,
            bounds,
        }
    }

    pub fn measure_text(&self, text: &str, style: &TextStyle) -> Size {
        Text::with_text_style(text, Point::zero(), self.default_char_style, *style)
            .bounding_box()
            .size
    }

    pub fn will_text_fit(&self, text: &str, position: Point, style: &TextStyle) -> bool {
        let text_size = self.measure_text(text, style);
        let text_bounds = Rectangle::new(position, text_size);

        // Check if text starts inside display bounds
        if !self.bounds.contains(position) {
            return false;
        }

        // Check if text ends inside display bounds
        text_bounds
            .bottom_right()
            .map(|p| self.bounds.contains(p))
            .unwrap_or(false)
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        position: Point,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.draw_text_with_style(text, position, &self.default_text_style.clone())
    }

    pub fn draw_text_clear(
        &mut self,
        text: &str,
        position: Point,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(TextError::DrawError)?;
        self.draw_text_with_style(text, position, &self.default_text_style.clone())
    }

    pub fn draw_text_clear_flush(
        &mut self,
        text: &str,
        position: Point,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.draw_text_clear(text, position)?;
        self.flush()
    }

    pub fn draw_text_with_style(
        &mut self,
        text: &str,
        position: Point,
        style: &TextStyle,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        if !self.will_text_fit(text, position, style) {
            return Err(TextError::DoesNotFit);
        }
        Text::with_text_style(text, position, self.default_char_style, *style)
            .draw(&mut self.display)
            .map_err(TextError::DrawError)
            .map(|_| ())
    }

    pub fn draw_text_with_style_clear(
        &mut self,
        text: &str,
        position: Point,
        style: &TextStyle,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(TextError::DrawError)?;
        self.draw_text_with_style(text, position, style)
    }

    pub fn draw_text_with_style_clear_flush(
        &mut self,
        text: &str,
        position: Point,
        style: &TextStyle,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.draw_text_with_style_clear(text, position, style)?;
        self.flush()
    }

    pub fn style_with_font(&self, font: &'a MonoFont<'a>) -> MonoTextStyle<'a, BinaryColor> {
        MonoTextStyleBuilder::new()
            .font(font)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build()
    }

    pub fn set_char_style(&mut self, style: MonoTextStyle<'a, BinaryColor>) {
        self.default_char_style = style;
    }

    pub fn set_text_color(&mut self, color: BinaryColor) {
        self.default_char_style.text_color = Some(color);
    }

    pub fn display_size(&self) -> Size {
        self.bounds.size
    }

    pub fn clear(&mut self) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(TextError::DrawError)
    }

    pub fn flush(&mut self) -> Result<(), TextError<DisplayError<DI, SIZE>>> {
        self.display.flush().map_err(TextError::DrawError)
    }
}
