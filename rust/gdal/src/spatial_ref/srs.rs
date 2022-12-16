/*
This file is part of the Building Aggregration Tool
Copyright (C) 2022 Novel-T

The Building Aggregration Tool is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/
use crate::utils::{_last_cpl_err, _last_null_pointer_err, _string};
use gdal_sys::{self, CPLErr, OGRCoordinateTransformationH, OGRErr, OGRSpatialReferenceH, OSRAxisMappingStrategy};
use libc::c_int;
use std::ffi::{CStr, CString};
use std::ptr;
use std::str::FromStr;

use crate::errors::*;
use anyhow::Result;

pub struct CoordTransform {
    inner: OGRCoordinateTransformationH,
    from: String,
    to: String,
}

impl Drop for CoordTransform {
    fn drop(&mut self) {
        unsafe { gdal_sys::OCTDestroyCoordinateTransformation(self.inner) };
        self.inner = ptr::null_mut();
    }
}

impl CoordTransform {
    pub fn new(sp_ref1: &SpatialRef, sp_ref2: &SpatialRef) -> Result<CoordTransform> {
        let c_obj = unsafe { gdal_sys::OCTNewCoordinateTransformation(sp_ref1.c_spatial_ref,
                                                                      sp_ref2.c_spatial_ref) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OCTNewCoordinateTransformation"))?;
        }
        Ok(CoordTransform {
            inner: c_obj,
            from: sp_ref1.authority().or_else(|_| sp_ref1.to_proj4())?,
            to: sp_ref2.authority().or_else(|_| sp_ref2.to_proj4())?,
        })
    }

    pub fn transform_point(&self, xy: &[f64; 2]) -> Result<[f64; 2]> {
        let mut x = [xy[0]];
        let mut y = [xy[1]];
        let mut z = [0.0];

        self.transform_coords(&mut x, &mut y, &mut z)?;

        Ok([x[0], y[0]])
    }

    pub fn transform_coords(&self, x: &mut [f64], y: &mut [f64], z: &mut [f64]) -> Result<()> {
        let nb_coords = x.len();
        assert_eq!(nb_coords, y.len());
        let ret_val = unsafe {
            gdal_sys::OCTTransform(
                self.inner,
                nb_coords as c_int,
                x.as_mut_ptr(),
                y.as_mut_ptr(),
                z.as_mut_ptr(),
            ) == 1
        };

        if ret_val {
            Ok(())
        } else {
            let err = _last_cpl_err(CPLErr::CE_Failure);
            let msg = if let ErrorKind::CplError { msg, .. } = err {
                if msg.trim().is_empty() {
                    None
                } else {
                    Some(msg)
                }
            } else {
                Err(err)?
            };
            Err(ErrorKind::InvalidCoordinateRange {
                from: self.from.clone(),
                to: self.to.clone(),
                msg,
            })?
        }
    }

    #[deprecated(since = "0.3.1", note = "use `transform_coords` instead")]
    pub fn transform_coord(&self, x: &mut [f64], y: &mut [f64], z: &mut [f64]) {
        self.transform_coords(x, y, z)
            .expect("Coordinate transform failed")
    }

    pub fn to_c_hct(&self) -> OGRCoordinateTransformationH {
        self.inner
    }
}

#[derive(Debug)]
pub struct SpatialRef {
    pub(crate) c_spatial_ref: OGRSpatialReferenceH
}

impl Drop for SpatialRef {
    fn drop(&mut self) {
        unsafe { gdal_sys::OSRRelease(self.c_spatial_ref) };
        self.c_spatial_ref = ptr::null_mut();
    }
}

impl Clone for SpatialRef {
    fn clone(&self) -> SpatialRef {
        let n_obj = unsafe { gdal_sys::OSRClone(self.c_spatial_ref) };
        SpatialRef{
            c_spatial_ref: n_obj
        }
    }
}

impl PartialEq for SpatialRef {
    fn eq(&self, other: &SpatialRef) -> bool {
        unsafe { gdal_sys::OSRIsSame(self.c_spatial_ref, other.c_spatial_ref) == 1 }
    }
}

impl SpatialRef {
    pub fn new() -> Result<SpatialRef> {
        let c_obj = unsafe { gdal_sys::OSRNewSpatialReference(ptr::null()) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OSRNewSpatialReference"))?;
        }
        Ok(SpatialRef{c_spatial_ref: c_obj})
    }

    pub fn from_definition(definition: &str) -> Result<SpatialRef> {
        let c_obj = unsafe { gdal_sys::OSRNewSpatialReference(ptr::null()) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OSRNewSpatialReference"))?;
        }
        let rv =
            unsafe { gdal_sys::OSRSetFromUserInput(c_obj, CString::new(definition)?.as_ptr()) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRSetFromUserInput",
            })?;
        }
        Ok(SpatialRef{c_spatial_ref: c_obj})
    }

    pub fn from_wkt(wkt: &str) -> Result<SpatialRef> {
        let c_str = CString::new(wkt)?;
        let c_obj = unsafe { gdal_sys::OSRNewSpatialReference(c_str.as_ptr()) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OSRNewSpatialReference"))?;
        }
        let mut r = SpatialRef{c_spatial_ref: c_obj};

        r.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

        Ok(r)
    }

    pub fn from_epsg(epsg_code: u32) -> Result<SpatialRef> {
        let null_ptr = ptr::null_mut();
        let c_obj = unsafe { gdal_sys::OSRNewSpatialReference(null_ptr) };
        let rv = unsafe { gdal_sys::OSRImportFromEPSG(c_obj, epsg_code as c_int) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRImportFromEPSG",
            })?
        } else {
            let mut r = SpatialRef{c_spatial_ref: c_obj};
            r.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);
            Ok(r)
        }
    }

    pub fn from_proj4(proj4_string: &str) -> Result<SpatialRef> {
        let c_str = CString::new(proj4_string)?;
        let null_ptr = ptr::null_mut();
        let c_obj = unsafe { gdal_sys::OSRNewSpatialReference(null_ptr) };
        let rv = unsafe { gdal_sys::OSRImportFromProj4(c_obj, c_str.as_ptr()) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRImportFromProj4",
            })?
        } else {
            let mut r = SpatialRef{c_spatial_ref: c_obj};
            r.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);
            Ok(r)
        }
    }

    pub fn from_esri(esri_wkt: &str) -> Result<SpatialRef> {
        let c_str = CString::new(esri_wkt)?;
        let mut ptrs = vec![c_str.as_ptr() as *mut i8, ptr::null_mut()];
        let null_ptr = ptr::null_mut();
        let c_obj = unsafe { gdal_sys::OSRNewSpatialReference(null_ptr) };
        let rv = unsafe { gdal_sys::OSRImportFromESRI(c_obj, ptrs.as_mut_ptr()) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRImportFromESRI",
            })?
        } else {
            let mut r = SpatialRef{c_spatial_ref: c_obj};
            r.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);
            Ok(r)
        }
    }

    pub(crate) fn from_c_obj(c_obj: OGRSpatialReferenceH) -> Result<SpatialRef> {
        let mut_c_obj = unsafe { gdal_sys::OSRClone(c_obj) };
        if mut_c_obj.is_null() {
            Err(_last_null_pointer_err("OSRClone"))?
        } else {
            Ok(SpatialRef{c_spatial_ref: mut_c_obj})
        }
    }

    pub fn to_wkt(&self) -> Result<String> {
        let mut c_wkt = ptr::null_mut();
        let rv = unsafe { gdal_sys::OSRExportToWkt(self.c_spatial_ref, &mut c_wkt) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRExportToWkt",
            })?
        } else {
            Ok(_string(c_wkt))
        }
    }

    pub fn morph_to_esri(&self) -> Result<()> {
        let rv = unsafe { gdal_sys::OSRMorphToESRI(self.c_spatial_ref) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRMorphToESRI",
            })?;
        }
        Ok(())
    }

    pub fn to_pretty_wkt(&self) -> Result<String> {
        let mut c_wkt = ptr::null_mut();
        let rv = unsafe { gdal_sys::OSRExportToPrettyWkt(self.c_spatial_ref, &mut c_wkt, false as c_int) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRExportToPrettyWkt",
            })?
        } else {
            Ok(_string(c_wkt))
        }
    }

    pub fn to_xml(&self) -> Result<String> {
        let mut c_raw_xml = ptr::null_mut();
        let rv = unsafe { gdal_sys::OSRExportToXML(self.c_spatial_ref, &mut c_raw_xml, ptr::null()) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRExportToXML",
            })?
        } else {
            Ok(_string(c_raw_xml))
        }
    }

    pub fn to_proj4(&self) -> Result<String> {
        let mut c_proj4str = ptr::null_mut();
        let rv = unsafe { gdal_sys::OSRExportToProj4(self.c_spatial_ref, &mut c_proj4str) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRExportToProj4",
            }
            .into())
        } else {
            Ok(_string(c_proj4str))
        }
    }

    pub fn auth_name(&self) -> Result<String> {
        let c_ptr = unsafe { gdal_sys::OSRGetAuthorityName(self.c_spatial_ref, ptr::null()) };
        if c_ptr.is_null() {
            Err(_last_null_pointer_err("SRGetAuthorityName"))?
        } else {
            Ok(_string(c_ptr))
        }
    }

    pub fn auth_code(&self) -> Result<i32> {
        let c_ptr = unsafe { gdal_sys::OSRGetAuthorityCode(self.c_spatial_ref, ptr::null()) };
        if c_ptr.is_null() {
            Err(_last_null_pointer_err("OSRGetAuthorityCode"))?;
        }
        let c_str = unsafe { CStr::from_ptr(c_ptr) };
        let epsg = i32::from_str(c_str.to_str()?);
        match epsg {
            Ok(n) => Ok(n),
            Err(_) => Err(ErrorKind::OgrError {
                err: OGRErr::OGRERR_UNSUPPORTED_SRS,
                method_name: "OSRGetAuthorityCode",
            })?,
        }
    }

    pub fn authority(&self) -> Result<String> {
        let c_ptr = unsafe { gdal_sys::OSRGetAuthorityName(self.c_spatial_ref, ptr::null()) };
        if c_ptr.is_null() {
            Err(_last_null_pointer_err("SRGetAuthorityName"))?;
        }
        let name = unsafe { CStr::from_ptr(c_ptr) }.to_str()?;
        let c_ptr = unsafe { gdal_sys::OSRGetAuthorityCode(self.c_spatial_ref, ptr::null()) };
        if c_ptr.is_null() {
            Err(_last_null_pointer_err("OSRGetAuthorityCode"))?;
        }
        let code = unsafe { CStr::from_ptr(c_ptr) }.to_str()?;
        Ok(format!("{}:{}", name, code))
    }

    pub fn auto_identify_epsg(&mut self) -> Result<()> {
        let rv = unsafe { gdal_sys::OSRAutoIdentifyEPSG(self.c_spatial_ref) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OSRAutoIdentifyEPSG",
            })?
        } else {
            Ok(())
        }
    }

    //#[cfg(feature = "gdal_3_0")]
    pub fn set_axis_mapping_strategy(&mut self, strategy: gdal_sys::OSRAxisMappingStrategy::Type) {
        unsafe {
            gdal_sys::OSRSetAxisMappingStrategy(self.c_spatial_ref, strategy);
        }
    }

    //#[cfg(feature = "gdal_3_0")]
    pub fn get_axis_mapping_strategy(&self) -> gdal_sys::OSRAxisMappingStrategy::Type {
        unsafe { gdal_sys::OSRGetAxisMappingStrategy(self.c_spatial_ref) }
    }


}
