use std::error::Error;
use std::path::PathBuf;

use gdal::raster::RasterBand;
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use gdal::{Dataset as GDALDataset, Metadata};

use crate::affine::Affine;
use crate::bounds::Bounds;

#[derive(Debug)]
pub struct Dataset {
    ds: GDALDataset,
    // crs: String,
    // transform: Affine,
    // width: u32,
    // height: u32,
    // TODO: dtype, nodata, bounds
}

impl Dataset {
    pub fn open(path: &PathBuf) -> Result<Dataset, Box<dyn Error>> {
        let d = GDALDataset::open(path)?;
        println!("dataset description: {:?}", d.description());

        let transform = d.geo_transform()?;
        // println!("transform: {:?}", transform);

        // also has spatial_ref object
        let crs = d.projection();
        // println!("crs: {}", crs);

        let (width, height) = d.raster_size();
        println!("Dimensions: {}, {}", width, height);

        let b: RasterBand = d.rasterband(1)?;
        // println!("rasterband description: {:?}", b.description());
        // println!("rasterband no_data_value: {:?}", b.no_data_value());
        // println!("rasterband type: {:?}", b.band_type());

        let dataset = Dataset {
            ds: d,
            // TODO: needs other fields
        };

        Ok(dataset)
    }

    pub fn bounds(&self) -> Result<Bounds, Box<dyn Error>> {
        let (width, height) = self.ds.raster_size();
        let transform = self.ds.geo_transform()?;
        let affine = Affine::from_gdal(&transform);

        let bounds = Bounds {
            xmin: affine.c,
            ymin: affine.f + affine.e * height as f64,
            xmax: affine.c + affine.a * width as f64,
            ymax: affine.f,
        };

        Ok(bounds)
    }

    fn transform_bounds(&self, crs: &SpatialRef) -> Result<Bounds, Box<dyn Error>> {
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
}
