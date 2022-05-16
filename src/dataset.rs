use libc::c_double;
use std::error::Error;
use std::ffi::CString;
use std::path::PathBuf;
use std::ptr::null_mut;

use gdal::raster::{GdalType, RasterBand, ResampleAlg};
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use gdal::Dataset as GDALDataset;
use gdal_sys::{GDALAutoCreateWarpedVRT, GDALDatasetH, GDALResampleAlg};

use crate::affine::Affine;
use crate::array::{all_equals, print_2d, set_all, shift};
use crate::bounds::Bounds;
use crate::tileid::TileID;
use crate::window::Window;

// D
pub struct Dataset {
    ds: GDALDataset,
}

impl Dataset {
    pub fn open(path: &PathBuf) -> Result<Dataset, Box<dyn Error>> {
        let d = GDALDataset::open(path)?;

        // let transform = d.geo_transform()?;

        // // also has spatial_ref object
        // let crs = d.projection();

        // let (width, height) = d.raster_size();
        // println!("Dimensions: {}, {}", width, height);

        // let b: RasterBand = d.rasterband(1)?;

        Ok(Dataset { ds: d })
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

        // TODO: options
        let vrt: GDALDatasetH = unsafe {
            GDALAutoCreateWarpedVRT(
                self.ds.c_dataset(),
                src_wkt.as_ptr(),
                target_wkt.as_ptr(),
                GDALResampleAlg::GRA_NearestNeighbour,
                0. as c_double,
                null_mut(),
            )
        };

        if vrt.is_null() {
            return Err(String::from("could not create WarpedVRT").into());
        }

        let gdal_dataset = unsafe { GDALDataset::from_c_dataset(vrt) };
        Ok(Dataset { ds: gdal_dataset })
    }

    pub fn merctor_vrt(&self) -> Result<Dataset, Box<dyn Error>> {
        return self.warped_vrt(&SpatialRef::from_epsg(3857)?);
    }

    pub fn band(&self, band_index: isize) -> Result<RasterBand, Box<dyn Error>> {
        Ok(self.ds.rasterband(band_index)?)
    }

    /// Read tile data into buffer
    ///
    /// # Returns
    /// Some(bool) if read is successful; value of bool indicates if tile has data
    /// None if there is an error
    pub fn read_tile<T: Copy + PartialEq + GdalType + std::fmt::Debug>(
        &self,
        band: &RasterBand,
        tile_id: TileID,
        tile_size: usize,
        buffer: &mut [T],
        nodata: T,
    ) -> Result<bool, Box<dyn Error>> {
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
        let right = (((vrt_bounds.xmax - tile_bounds.xmax) / xres).round()).max(0.);
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

        println!(
            "Debug tile={:?}: window=({},{}), read_dims=({},{}), dims=({},{}=>{})",
            tile_id,
            x_offset,
            y_offset,
            read_width,
            read_height,
            width,
            height,
            width * height
        );
        println!("buffer size: {}", buffer[0..(width * height)].len());

        if read_width <= 0 || read_height <= 0 {
            println!("Tile is outside dataset extent");
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
            println!("Tile is empty");
            return Ok(false);
        }

        // println!("before");
        // print_2d(buffer, (width, height));

        if width < tile_size || height < tile_size {
            // partial tile
            shift(
                buffer,
                (width, height),
                (tile_size, tile_size),
                (left as usize, top as usize),
                nodata,
            );

            // println!("\n\nafter");
            // print_2d(buffer, (tile_size, tile_size));
        }

        Ok(true)
    }
}
