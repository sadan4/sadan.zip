use std::ops::{Deref, DerefMut};

use wasm_bindgen::{JsCast as _, JsValue, prelude::wasm_bindgen};
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, window};

use crate::util::{get_computed_style, get_document, get_window};

struct Snowflake {
    x: f32,
    y: f32,
    radius: u8,
    wind: f32,
    opacity: f32,
    reverse_chance: f32,
    fade_out_point: f32,
    fade_speed: f32,
}

#[wasm_bindgen]
struct SnowCanvasOptions {
    density: u8,
    min_speed: f32,
    max_speed: f32,
    min_size: u8,
    max_size: u8,
    wind_strength: f32,
}

impl SnowCanvasOptions {
    fn new(density: u8, min_speed: f32, max_speed: f32, min_size: u8, max_size: u8, wind_strength: f32) -> Self {
        Self {
            density,
            min_speed,
            max_speed,
            min_size,
            max_size,
            wind_strength,
        }
    }
}

struct SnowCanvasData {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    fill_style: String,
    opts: SnowCanvasOptions,
    snowflakes: Vec<Snowflake>,
}

#[wasm_bindgen]
pub struct SnowCanvas {
    data: SnowCanvasData,
}

impl From<SnowCanvasData> for SnowCanvas {
    fn from(data: SnowCanvasData) -> Self {
        Self { data }
    }
}

const SNOW_COLOR_VAR: &'static str = "--color-fg-500";

#[wasm_bindgen]
impl SnowCanvas {
    #[wasm_bindgen(constructor)]
    pub fn try_new(el: HtmlCanvasElement) -> Result<Self, JsValue> {
        let ctx = el
            .get_context("2d")?
            .ok_or("Could not get 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "invalid type")?;
        let document_element = get_document().document_element().unwrap();
        let snow_color = get_computed_style(&document_element)
            .unwrap()
            .unwrap()
            .get_property_value(SNOW_COLOR_VAR)
            .unwrap();

        Err("Foobar".into())
    }
    fn resize_canvas(&self) {
        let window = get_window();
        let inner_width = window
            .inner_width()
            .map(TryInto::<u64>::try_into)
            .unwrap()
            .unwrap();
        self.data.canvas.set_width(inner_width as u32);

        let inner_height = window
            .inner_height()
            .map(TryInto::<u64>::try_into)
            .unwrap()
            .unwrap();
        self.data.canvas.set_height(inner_height as u32);
    }
}
