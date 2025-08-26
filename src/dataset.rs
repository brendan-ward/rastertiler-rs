#![allow(unused)]

use libc::c_double;
use std::error::Error;
use std::ffi::CString;
use std::path::PathBuf;

use gdal::cpl::CslStringList;
use gdal::raster::{Buffer, GdalType, RasterBand, RasterCreationOptions, ResampleAlg};
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use gdal::{Dataset as GDALDataset, DatasetOptions, DriverManager};
use gdal_sys::{GDALAutoCreateWarpedVRT, GDALCreateWarpOptions, GDALDatasetH, GDALResampleAlg};

use crate::affine::Affine;
use crate::array::{all_equals, set_all, shift};
use crate::bounds::Bounds;
use crate::tileid::TileID;
use crate::window::Window;

pub struct Dataset {
    ds: GDALDataset,
}

impl Dataset {
    pub fn open(path: &PathBuf, disable_overviews: bool) -> Result<Dataset, Box<dyn Error>> {
        let mut options = DatasetOptions::default();

        if disable_overviews {
            options.open_options = Some(&["OVERVIEW_LEVEL=NONE"]);
        }

        Ok(Dataset {
            ds: GDALDataset::open_ex(path, options)?,
        })
    }

    pub fn bounds(&self) -> Result<Bounds, Box<dyn Error>> {
        let (width, height) = self.ds.raster_size();
        let transform = self.ds.geo_transform()?;
        let affine = Affine::from_gdal(&transform);

        Ok(Bounds {
            xmin: affine.c,
            ymin: affine.f + affine.e * height as f64,
            xmax: affine.c + affine.a * width as f64,
            ymax: affine.f,
        })
    }

    pub fn transform_bounds(&self, crs: &SpatialRef) -> Result<Bounds, Box<dyn Error>> {
        let bounds = self.bounds()?;
        let src_crs = self.ds.spatial_ref()?;
        let transform = CoordTransform::new(&src_crs, crs).unwrap();
        let out_bounds = transform
            .transform_bounds(&[bounds.xmin, bounds.ymin, bounds.xmax, bounds.ymax], 21)?;

        Ok(Bounds {
            xmin: out_bounds[0],
            ymin: out_bounds[1],
            xmax: out_bounds[2],
            ymax: out_bounds[3],
        })
    }

    pub fn geo_bounds(&self) -> Result<Bounds, Box<dyn Error>> {
        self.transform_bounds(&SpatialRef::from_definition("OGC:CRS84")?)
    }

    pub fn mercator_bounds(&self) -> Result<Bounds, Box<dyn Error>> {
        self.transform_bounds(&SpatialRef::from_epsg(3857)?)
    }

    // TODO: migrate to georust/gdal
    fn warped_vrt(&self, sp_ref: &SpatialRef) -> Result<Dataset, Box<dyn Error>> {
        let src_wkt = CString::new(self.ds.spatial_ref()?.to_wkt()?)?;
        let target_wkt = CString::new(sp_ref.to_wkt()?)?;

        let mut str_opts = CslStringList::new();
        str_opts.set_name_value("INIT_DEST", "NO_DATA")?;
        str_opts.set_name_value("NUM_THREADS", "1")?;

        let mut options = unsafe { GDALCreateWarpOptions() };
        // use 2GB memory for warping (doesn't seem to help)
        unsafe { (*options).dfWarpMemoryLimit = 2048. * 1024. * 1024. };
        unsafe {
            (*options).papszWarpOptions = str_opts.as_ptr();
        }

        let vrt: GDALDatasetH = unsafe {
            GDALAutoCreateWarpedVRT(
                self.ds.c_dataset(),
                src_wkt.as_ptr(),
                target_wkt.as_ptr(),
                GDALResampleAlg::GRA_NearestNeighbour,
                0. as c_double,
                options,
            )
        };

        if vrt.is_null() {
            return Err(String::from("could not create WarpedVRT").into());
        }

        let gdal_dataset = unsafe { GDALDataset::from_c_dataset(vrt) };
        Ok(Dataset { ds: gdal_dataset })
    }

    pub fn mercator_vrt(&self) -> Result<Dataset, Box<dyn Error>> {
        self.warped_vrt(&SpatialRef::from_epsg(3857)?)
    }

    pub fn band(&self, band_index: usize) -> Result<RasterBand<'_>, Box<dyn Error>> {
        Ok(self.ds.rasterband(band_index)?)
    }

    /// Read tile data into buffer
    ///
    /// # Returns
    /// Some(bool) if read is successful; value of bool indicates if tile has data
    /// None if there is an error
    pub fn read_tile<T: Copy + PartialEq + Ord + GdalType + std::fmt::Debug>(
        &self,
        band: &RasterBand,
        tile_id: TileID,
        tile_size: u16,
        buffer: &mut [T],
        nodata: T,
    ) -> Result<bool, Box<dyn Error>> {
        let tile_size = tile_size as usize;
        let size = tile_size as f64;

        let (vrt_width, vrt_height) = self.ds.raster_size();
        let vrt_width_f = vrt_width as f64;
        let vrt_height_f = vrt_height as f64;
        let vrt_transform = Affine::from_gdal(&self.ds.geo_transform()?);
        let vrt_bounds = self.bounds()?;

        let tile_bounds = tile_id.mercator_bounds();
        let window = Window::from_bounds(&vrt_transform, &tile_bounds);
        let tile_transform = window
            .transform(&vrt_transform)
            .scale(window.width / size, window.height / size);

        let (xres, yres) = tile_transform.resolution();

        let left = (((vrt_bounds.xmin - tile_bounds.xmin) / xres).round()).max(0.);
        let right = (((tile_bounds.xmax - vrt_bounds.xmax) / xres).round()).max(0.);
        let bottom = (((vrt_bounds.ymin - tile_bounds.ymin) / yres).round()).max(0.);
        let top = (((tile_bounds.ymax - vrt_bounds.ymax) / yres).round()).max(0.);

        // calculate width and height in coordinates of VRT
        let width = (size - left - right).round() as usize;
        let height = (size - top - bottom).round() as usize;

        let x_offset = (((window.x_offset).max(0.)).min(vrt_width_f)).round();
        let y_offset = (((window.y_offset).max(0.)).min(vrt_height_f)).round();
        let x_stop = ((window.x_offset + window.width).min(vrt_width_f)).max(0.);
        let y_stop = ((window.y_offset + window.height).min(vrt_height_f)).max(0.);

        let read_width = ((x_stop - x_offset) + 0.5).floor() as usize;
        let read_height = ((y_stop - y_offset) + 0.5).floor() as usize;

        if read_width == 0 || read_height == 0 {
            // to data available within extent of dataset
            return Ok(false);
        }

        // reset buffer to NODATA
        set_all(buffer, nodata);

        // read full or partial tile
        band.read_into_slice(
            (x_offset as isize, y_offset as isize),
            (read_width, read_height),
            (width, height),
            &mut buffer[0..(width * height)],
            Some(ResampleAlg::NearestNeighbour),
        )?;

        if all_equals(buffer, nodata) {
            return Ok(false);
        }

        if left > 0. || top > 0. || width < tile_size {
            // partial tile
            shift(
                buffer,
                (width, height),
                (tile_size, tile_size),
                (left as usize, top as usize),
                nodata,
            );
        }

        Ok(true)
    }
}

pub fn write_raster<T: GdalType + Copy>(
    path: String,
    width: usize,
    height: usize,
    transform: &Affine,
    spatialref: &SpatialRef,
    data: Vec<T>,
    nodata: f64,
) -> Result<(), Box<dyn Error>> {
    let driver = DriverManager::get_driver_by_name("GTiff").unwrap();
    let options = RasterCreationOptions::from_iter([
        "TILED=YES",
        "BLOCKXSIZE=256",
        "BLOCKYSIZE=256",
        "COMPRESS=LZW",
        "INTERLEAVE=BAND",
    ]);
    let mut dataset = driver
        .create_with_band_type_with_options::<T, _>(path, width, height, 1, &options)
        .unwrap();
    dataset.set_geo_transform(&transform.to_gdal())?;
    dataset.set_spatial_ref(spatialref)?;

    let mut band = dataset.rasterband(1)?;
    band.set_no_data_value(Some(nodata))?;

    let mut raster = Buffer::new((width, height), data);

    band.write((0, 0), (width, height), &mut raster)?;

    Ok(())
}
