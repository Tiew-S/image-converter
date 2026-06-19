use anyhow::{Result, anyhow};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(PartialEq, Clone, Copy)]
pub enum ConversionState {
    Untouched,
    Processing,
    Success,
    Fail,
}

#[derive(PartialEq, Clone, Copy)]
pub enum OtherFormats {
    Pdf,
    Svg,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ImageFormat {
    Normal(image::ImageFormat),
    Other(OtherFormats),
}

#[derive(PartialEq, Clone)]
pub struct Image {
    pub path: PathBuf,
    pub format: ImageFormat,
}

impl Image {
    fn from_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();
        Ok(Self {
            format: match path
                .extension()
                .ok_or(anyhow!("Couldn't get extension?!"))?
                .to_str()
                .ok_or(anyhow!(
                    "Couldn't convert extension to &str, what the fuck?"
                ))? {
                "svg" => ImageFormat::Other(OtherFormats::Svg),
                "pdf" => ImageFormat::Other(OtherFormats::Pdf),
                ext => ImageFormat::Normal(
                    image::ImageFormat::from_extension(ext)
                        .ok_or(anyhow!("What kind of extension is {ext}?!"))?,
                ),
            },
            path,
        })
    }

    pub fn convert(&self, img_format: &ImageFormat) -> Result<()> {
        match img_format {
            ImageFormat::Normal(fmt) => {
                let img = image::ImageReader::open(&self.path)?.decode()?;
                img.save_with_format(
                    &self.path.with_extension(
                        fmt.extensions_str()
                            .first()
                            .ok_or(anyhow!("No &str for format {:?}, somehow", fmt))?,
                    ),
                    fmt.clone(),
                )?;
                Ok(())
            }
            ImageFormat::Other(fmt) => match fmt {
                OtherFormats::Pdf => todo!("TODO: PDF conversion"),
                OtherFormats::Svg => todo!("TODO: SVG conversion"),
            },
        }
    }
}

pub struct ImageConverterOptions {
    pdf_render_dpi: u64,
    pdf_render_to_separate_pages: bool,
}

impl Default for ImageConverterOptions {
    fn default() -> Self {
        Self {
            pdf_render_dpi: 300,
            pdf_render_to_separate_pages: true,
        }
    }
}

pub struct ImageConverter {
    pub images: Vec<(Image, ConversionState)>,
    pub options: ImageConverterOptions,
}

impl ImageConverter {
    pub fn new() -> Self {
        Self {
            images: vec![],
            options: ImageConverterOptions::default(),
        }
    }

    pub fn new_with_options(options: ImageConverterOptions) -> Self {
        Self {
            images: vec![],
            options,
        }
    }

    // pub fn convert<T: 'static>(&mut self, cx: &mut gpui::Context<T>, img_format: ImageFormat) {
    //     self.images
    //         .iter_mut()
    //         .for_each(|(image, conversion_state)| {
    //             *conversion_state = ConversionState::Processing;
    //             cx.notify();
    //             match image.convert(&img_format) {
    //                 Ok(_) => *conversion_state = ConversionState::Success,
    //                 Err(_) => *conversion_state = ConversionState::Fail,
    //             }
    //             cx.notify();
    //         });
    // }

    pub fn add_image_from_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if self
            .images
            .iter()
            .map(|(i, _)| &i.path)
            .find(|p| p.as_path() == path.as_ref())
            .is_some()
        {
            return Err(anyhow!("Image paths must be unique"));
        }
        self.images.push((
            Image::from_path(path.as_ref().to_path_buf())?,
            ConversionState::Untouched,
        ));
        Ok(())
    }
}
