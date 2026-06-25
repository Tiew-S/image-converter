use anyhow::{Result, anyhow};
use gpui::Context;
use image::{GenericImage, Rgba};
use pdfium_bind::*;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use serde::{Serialize, Deserialize};
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub image_convertion_options: ImageConvertionOptions
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            image_convertion_options: Default::default()
        }
    }
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

    pub fn convert(
        &self,
        img_format: &image::ImageFormat,
        options: &ImageConvertionOptions,
    ) -> Result<()> {
        match self.format {
            ImageFormat::Normal(_) => {
                let img = image::ImageReader::open(&self.path)?.decode()?;
                img.save_with_format(
                    &self.path.with_extension(
                        img_format
                            .extensions_str()
                            .first()
                            .ok_or(anyhow!("No &str for format {:?}, somehow", img_format))?,
                    ),
                    img_format.clone(),
                )?;
                Ok(())
            }
            ImageFormat::Other(fmt) => match fmt {
                OtherFormats::Pdf => {
                    let doc = PdfDocument::open(&self.path).map_err(|s| anyhow!(s))?;

                    let extension = img_format
                        .extensions_str()
                        .first()
                        .ok_or(anyhow!("No &str for format {:?}, somehow", img_format))?;
                    let doc_page_count = doc.page_count();
                    let renders: Vec<((Vec<u8>, i32, i32), isize)> = (0..doc.page_count())
                        .filter_map(|index| {
                            let render = doc.render_page(index, options.pdf_render_dpi as f32);

                            if render.is_err() {
                                dbg!("Couln't render");
                                return None;
                            }
                            Some((render.unwrap(), index))
                        })
                        .collect();
                    if options.pdf_render_to_separate_pages {
                        let mut pdf_folder = self.path.clone();

                        pdf_folder.pop();
                        pdf_folder.push(self.path.file_stem().ok_or(anyhow!("No file stem"))?);
                        fs::create_dir(&pdf_folder).ok();
                        renders.par_iter().for_each(|(render, index)| {
                            let mut img = image::DynamicImage::new(
                                render.1 as u32,
                                render.2 as u32,
                                image::ColorType::Rgba8,
                            );

                            for i in 0..(render.1 * render.2) {
                                img.put_pixel(
                                    (i % render.1) as u32,
                                    (i / render.1) as u32,
                                    Rgba([
                                        *(render.0.get((4 * i + 0) as usize).unwrap()),
                                        *(render.0.get((4 * i + 1) as usize).unwrap()),
                                        *(render.0.get((4 * i + 2) as usize).unwrap()),
                                        *(render.0.get((4 * i + 3) as usize).unwrap()),
                                    ]),
                                );
                            }

                            let Ok(file_name) = self
                                .path
                                .file_stem()
                                .ok_or(anyhow!("No filename?"))
                                .and_then(|s| s.to_str().ok_or(anyhow!("Couldn't convert to &str")))
                                .and_then(|s| {
                                    Ok(s.to_string()
                                        + format!(" (page {} of {})", index + 1, doc_page_count)
                                            .as_str())
                                })
                            else {
                                return;
                            };

                            let path = pdf_folder.join(file_name).with_extension(extension);
                            let _ = img.save_with_format(path, img_format.clone());
                        });
                    } else {
                        let mut img = image::DynamicImage::new(
                            renders
                                .iter()
                                .max_by_key(|(r, _)| r.1)
                                .and_then(|r| Some(r.0.1))
                                .unwrap_or(0) as u32,
                            renders.iter().map(|(r, _)| r.2).sum::<i32>() as u32,
                            image::ColorType::Rgba8,
                        );
                        let mut height_step = 0;
                        for (render, _) in renders {
                            for i in 0..(render.1 * render.2) {
                                img.put_pixel(
                                    (i % render.1) as u32,
                                    height_step + (i / render.1) as u32,
                                    Rgba([
                                        *(render.0.get((4 * i + 0) as usize).unwrap()),
                                        *(render.0.get((4 * i + 1) as usize).unwrap()),
                                        *(render.0.get((4 * i + 2) as usize).unwrap()),
                                        *(render.0.get((4 * i + 3) as usize).unwrap()),
                                    ]),
                                );
                            }
                            height_step += render.2 as u32;
                        }

                        let path = self.path.with_extension(extension);
                        let _ = img.save_with_format(path, img_format.clone());
                    }

                    Ok(())
                }
                OtherFormats::Svg => todo!("TODO: SVG conversion"),
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ImageConvertionOptions {
    pub svg_render_dpi: u64,
    pub pdf_render_dpi: u64,
    pub pdf_render_to_separate_pages: bool,
}

impl Default for ImageConvertionOptions {
    fn default() -> Self {
        Self {
            svg_render_dpi: 300,
            pdf_render_dpi: 100,
            pdf_render_to_separate_pages: true
        }
    }
}


#[derive(Clone)]
pub struct ImageConverter {
    images: Vec<Image>,
    pub options: ImageConvertionOptions,
}

impl ImageConverter {
    pub fn new() -> Self {
        Self {
            images: vec![],
            options: ImageConvertionOptions::default(),
        }
    }

    pub fn get_images(&self) -> &Vec<Image> {
        &self.images
    }

    pub fn add_image_from_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if self
            .images
            .iter()
            .map(|i| &i.path)
            .find(|p| p.as_path() == path.as_ref())
            .is_some()
        {
            return Err(anyhow!("Image paths must be unique"));
        }
        self.images
            .push(Image::from_path(path.as_ref().to_path_buf())?);
        Ok(())
    }

    pub fn remove_image(&mut self, index: usize) {
        self.images.remove(index);
    }
}
