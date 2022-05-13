use libc::c_double;
use std::error::Error;
use std::ffi::CString;
use std::path::PathBuf;
use std::ptr::null_mut;

use gdal::raster::{Buffer, GdalType, RasterBand};
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use gdal::Dataset as GDALDataset;
use gdal_sys::{GDALAutoCreateWarpedVRT, GDALDatasetH, GDALResampleAlg};

use crate::affine::Affine;
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
        let bounds = self.bounds()?;
        self.transform_bounds(&SpatialRef::from_definition("OGC:CRS84")?)
    }

    pub fn mercator_bounds(&self) -> Result<Bounds, Box<dyn Error>> {
        let bounds = self.bounds()?;
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

    // Result<Buffer<T>, Box<dyn Error>>
    pub fn read_tile<T: Copy + GdalType>(
        &self,
        band: &RasterBand,
        tile_id: TileID,
        tile_size: u16,
        buffer: &mut [T],
    ) -> Result<(), Box<dyn Error>> {
        let size = tile_size as f64;

        let (vrt_width, vrt_height) = self.ds.raster_size();
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

        let width = (size - left - right).round() as i32;
        let height = (size - top - bottom).round() as i32;

        // FIXME:
        Ok(())

        // TODO: other stuff
        // rasterband.read_into_slice::<u8>((20, 30), (2, 3), (2, 3),TODO:buffer, None)
    }
}
