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
use super::{
    Dataset, Driver, Feature, FeatureIterator, FieldValue, Geometry, OGRFieldType,
    OGRwkbGeometryType,
};
use crate::assert_almost_eq;
use crate::spatial_ref::SpatialRef;
use std::path::Path;

macro_rules! fixture {
    ($name:expr) => {
        /*&Path::new(file!())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")*/
            Path::new("/rust/gdal/fixtures/")
            //.as_path()
            .join($name)
            .as_path().to_str().unwrap()
    };
}

#[test]
fn test_layer_count() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    assert_eq!(ds.count(), 1);
}

#[test]
fn test_layer_extent() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    let layer = ds.layer(0).unwrap();
    assert!(layer.get_extent(false).is_err());
    let extent = layer.get_extent(true).unwrap();
    assert_almost_eq(extent.MinX, 26.100768);
    assert_almost_eq(extent.MaxX, 26.103515);
    assert_almost_eq(extent.MinY, 44.429858);
    assert_almost_eq(extent.MaxY, 44.431818);
}

#[test]
fn test_layer_spatial_reference() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    let layer = ds.layer(0).unwrap();
    let srs = layer.spatial_reference().unwrap();
    assert_eq!(srs.auth_code().unwrap(), 4326);
}

fn with_features<F>(name: &str, f: F)
where
    F: Fn(FeatureIterator),
{
    let ds = Dataset::open(fixture!(name)).unwrap();
    let layer = ds.layer(0).unwrap();
    f(layer.features());
}

fn with_first_feature<F>(name: &str, f: F)
where
    F: Fn(Feature),
{
    with_features(name, |mut features| f(features.next().unwrap()));
}

#[test]
fn test_iterate_features() {
    with_features("roads.geojson", |features| {
        assert_eq!(features.count(), 21);
    });
}

#[test]
fn test_string_field() {
    with_features("roads.geojson", |mut features| {
        let feature = features.next().unwrap();
        assert_eq!(
            feature.field("highway").unwrap().into_string(),
            Some("footway".to_string())
        );
        assert_eq!(
            features
                .filter(|field| {
                    let highway = field.field("highway").unwrap().into_string();
                    highway == Some("residential".to_string())
                })
                .count(),
            2
        );
    });
}

#[test]
fn test_float_field() {
    with_first_feature("roads.geojson", |feature| {
        assert_almost_eq(
            feature.field("sort_key").unwrap().into_real().unwrap(),
            -9.0,
        );
    });
}

#[test]
fn test_missing_field() {
    with_first_feature("roads.geojson", |feature| {
        assert!(feature.field("no such field").is_err());
    });
}

#[test]
fn test_geom_accessors() {
    with_first_feature("roads.geojson", |feature| {
        let geom = feature.geometry().as_geom();
        assert_eq!(geom.geometry_type(), OGRwkbGeometryType::wkbLineString);
        let coords = geom.get_point_vec();
        assert_eq!(
            coords,
            [
                [26.1019276, 44.4302748],
                [26.1019382, 44.4303191],
                [26.1020002, 44.4304202]
            ]
        );
        assert_eq!(geom.geometry_count(), 0);

        let geom = feature.geometry_by_index(0).unwrap().as_geom();
        assert_eq!(geom.geometry_type(), OGRwkbGeometryType::wkbLineString);
        assert!(feature.geometry_by_index(1).is_err());
        let geom = feature.geometry_by_name("");
        assert!(!geom.is_err());
        let geom = feature.geometry_by_name("").unwrap().as_geom();
        assert_eq!(geom.geometry_type(), OGRwkbGeometryType::wkbLineString);
        assert!(feature.geometry_by_name("FOO").is_err());
    });
}

#[test]
fn test_wkt() {
    with_first_feature("roads.geojson", |feature| {
        let wkt = feature.geometry().as_geom().wkt().unwrap();
        let wkt_ok = format!(
            "{}{}",
            "LINESTRING (26.1019276 44.4302748,", "26.1019382 44.4303191,26.1020002 44.4304202)"
        );
        assert_eq!(wkt, wkt_ok);
    });
}

#[test]
fn test_json() {
    with_first_feature("roads.geojson", |feature| {
        let json = feature.geometry().as_geom().json();
        let json_ok = format!(
            "{}{}{}{}",
            "{ \"type\": \"LineString\", \"coordinates\": [ ",
            "[ 26.1019276, 44.4302748 ], ",
            "[ 26.1019382, 44.4303191 ], ",
            "[ 26.1020002, 44.4304202 ] ] }"
        );
        assert_eq!(json.unwrap(), json_ok);
    });
}

#[test]
fn test_schema() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    let layer = ds.layer(0).unwrap();
    // The layer name is "roads" in GDAL 2.2
    assert!(layer.name() == "OGRGeoJSON" || layer.name() == "roads");
    let name_list = layer
        .layer_definition()
        .fields()
        .map(|f| (f.name(), f.field_type()))
        .collect::<Vec<_>>();
    let ok_names_types = vec![
        ("id", OGRFieldType::OFTString),
        ("kind", OGRFieldType::OFTString),
        ("sort_key", OGRFieldType::OFTReal),
        ("is_link", OGRFieldType::OFTString),
        ("is_tunnel", OGRFieldType::OFTString),
        ("is_bridge", OGRFieldType::OFTString),
        ("railway", OGRFieldType::OFTString),
        ("highway", OGRFieldType::OFTString),
    ]
    .iter()
    .map(|s| (s.0.to_string(), s.1))
    .collect::<Vec<_>>();
    assert_eq!(name_list, ok_names_types);
}

#[test]
fn test_geom_fields() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    let layer = ds.layer(0).unwrap();
    let name_list = layer
        .layer_definition()
        .geom_fields()
        .map(|f| (f.name(), f.field_type()))
        .collect::<Vec<_>>();
    let ok_names_types = vec![("", OGRwkbGeometryType::wkbLineString)]
        .iter()
        .map(|s| (s.0.to_string(), s.1.clone()))
        .collect::<Vec<_>>();
    assert_eq!(name_list, ok_names_types);

    let layer_def = layer.layer_definition();
    let geom_field = layer_def.geom_fields().next().unwrap();
    let mut spatial_ref2 = SpatialRef::from_epsg(4326).unwrap();
    //#[cfg(feature = "gdal_3_0")]
    spatial_ref2.set_axis_mapping_strategy(0);

    assert!(geom_field.spatial_ref().unwrap() == spatial_ref2);
}

#[test]
fn test_get_layer_by_name() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    // The layer name is "roads" in GDAL 2.2
    if let Ok(layer) = ds.layer_by_name("OGRGeoJSON") {
        assert_eq!(layer.name(), "OGRGeoJSON");
    }
    if let Ok(layer) = ds.layer_by_name("roads") {
        assert_eq!(layer.name(), "roads");
    };
}

#[test]
fn test_create_bbox() {
    let bbox = Geometry::bbox(-27., 33., 52., 85.).unwrap();
    assert_eq!(bbox.json().unwrap(), "{ \"type\": \"Polygon\", \"coordinates\": [ [ [ -27.0, 85.0 ], [ 52.0, 85.0 ], [ 52.0, 33.0 ], [ -27.0, 33.0 ], [ -27.0, 85.0 ] ] ] }");
}

#[test]
fn test_spatial_filter() {
    let ds = Dataset::open(fixture!("roads.geojson")).unwrap();
    let layer = ds.layer(0).unwrap();
    assert_eq!(layer.features().count(), 21);

    let bbox = Geometry::bbox(26.1017, 44.4297, 26.1025, 44.4303).unwrap();
    layer.set_spatial_filter(&bbox);
    assert_eq!(layer.features().count(), 7);

    layer.clear_spatial_filter();
    assert_eq!(layer.features().count(), 21);
}

#[test]
fn test_convex_hull() {
    let star = "POLYGON ((0 1,3 1,1 3,1.5 0.0,2 3,0 1))";
    let hull = "POLYGON ((1.5 0.0,0 1,1 3,2 3,3 1,1.5 0.0))";
    assert_eq!(
        Geometry::from_wkt(star)
            .unwrap()
            .convex_hull()
            .unwrap()
            .wkt()
            .unwrap(),
        hull
    );
}

#[test]
fn test_write_features() {
    use std::fs;

    {
        let driver = Driver::get("GeoJSON").unwrap();
        let mut ds = driver.create(fixture!("output.geojson")).unwrap();
        let mut layer = ds.create_layer().unwrap();
        layer
            .create_defn_fields(&[
                ("Name", OGRFieldType::OFTString),
                ("Value", OGRFieldType::OFTReal),
                ("Int_value", OGRFieldType::OFTInteger),
            ])
            .unwrap();
        layer
            .create_feature_fields(
                Geometry::from_wkt("POINT (1 2)").unwrap(),
                &["Name", "Value", "Int_value"],
                &[
                    FieldValue::StringValue("Feature 1".to_string()),
                    FieldValue::RealValue(45.78),
                    FieldValue::IntegerValue(1),
                ],
            )
            .unwrap();
        // dataset is closed here
    }

    let ds = Dataset::open(fixture!("output.geojson")).unwrap();
    fs::remove_file(fixture!("output.geojson")).unwrap();
    let layer = ds.layer(0).unwrap();
    let ft = layer.features().next().unwrap();
    assert_eq!(ft.geometry().as_geom().wkt().unwrap(), "POINT (1 2)");
    assert_eq!(
        ft.field("Name").unwrap().into_string(),
        Some("Feature 1".to_string())
    );
    assert_eq!(ft.field("Value").unwrap().into_real(), Some(45.78));
    assert_eq!(ft.field("Int_value").unwrap().into_int(), Some(1));
}
