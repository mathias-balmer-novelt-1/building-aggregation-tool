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
use crate::{CoordDimensions, CoordSeq, Geometry as GGeometry};
use error::Error;
use geo_types::{Coordinate, LineString, MultiPolygon, Point, Polygon};

use std;
use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};

fn create_coord_seq_from_vec<'a, 'b>(coords: &'a [Coordinate<f64>]) -> Result<CoordSeq<'b>, Error> {
    create_coord_seq(coords.iter(), coords.len())
}

fn create_coord_seq<'a, 'b, It>(points: It, len: usize) -> Result<CoordSeq<'b>, Error>
where
    It: Iterator<Item = &'a Coordinate<f64>>,
{
    let mut coord_seq =
        CoordSeq::new(len as u32, CoordDimensions::TwoD).expect("failed to create CoordSeq");
    for (i, p) in points.enumerate() {
        coord_seq.set_x(i, p.x)?;
        coord_seq.set_y(i, p.y)?;
    }
    Ok(coord_seq)
}

impl<'a, 'b> TryFrom<&'a Point<f64>> for GGeometry<'b> {
    type Error = Error;

    fn try_from(other: &'a Point<f64>) -> Result<GGeometry<'b>, Self::Error> {
        let coord_seq = create_coord_seq(std::iter::once(&other.0), 1)?;

        GGeometry::create_point(coord_seq)
    }
}

impl<'a> TryFrom<Point<f64>> for GGeometry<'a> {
    type Error = Error;

    fn try_from(other: Point<f64>) -> Result<GGeometry<'a>, Self::Error> {
        GGeometry::try_from(&other)
    }
}

impl<'a, T: Borrow<Point<f64>>> TryFrom<&'a [T]> for GGeometry<'a> {
    type Error = Error;

    fn try_from(other: &'a [T]) -> Result<GGeometry<'a>, Self::Error> {
        let geom_points = other
            .into_iter()
            .map(|p| p.borrow().try_into())
            .collect::<Result<Vec<_>, _>>()?;

        GGeometry::create_multipoint(geom_points)
    }
}

impl<'a, 'b> TryFrom<&'a LineString<f64>> for GGeometry<'b> {
    type Error = Error;

    fn try_from(other: &'a LineString<f64>) -> Result<GGeometry<'b>, Self::Error> {
        let coord_seq = create_coord_seq_from_vec(other.0.as_slice())?;

        GGeometry::create_line_string(coord_seq)
    }
}

impl<'a> TryFrom<LineString<f64>> for GGeometry<'a> {
    type Error = Error;

    fn try_from(other: LineString<f64>) -> Result<GGeometry<'a>, Self::Error> {
        GGeometry::try_from(&other)
    }
}

// rust geo does not have the distinction LineString/LineRing, so we create a wrapper

struct LineRing<'a>(&'a LineString<f64>);

/// Convert a geo_types::LineString to a geos LinearRing
/// a LinearRing should be closed so cloase the geometry if needed
impl<'a, 'b> TryFrom<LineRing<'a>> for GGeometry<'b> {
    type Error = Error;

    fn try_from(other: LineRing<'a>) -> Result<GGeometry<'b>, Self::Error> {
        let points = &(other.0).0;
        let nb_points = points.len();
        if nb_points > 0 && nb_points < 3 {
            return Err(Error::InvalidGeometry(
                "impossible to create a LinearRing, A LinearRing must have at least 3 coordinates"
                    .into(),
            ));
        }

        // if the geom is not closed we close it
        let is_closed = nb_points > 0 && points.first() == points.last();
        // Note: we also need to close a 2 points closed linearring, cf test closed_2_points_linear_ring
        let need_closing = nb_points > 0 && (!is_closed || nb_points == 3);
        let coord_seq = if need_closing {
            create_coord_seq(
                points.iter().chain(std::iter::once(&points[0])),
                nb_points + 1,
            )?
        } else {
            create_coord_seq(points.iter(), nb_points)?
        };
        GGeometry::create_linear_ring(coord_seq)
    }
}

impl<'a, 'b> TryFrom<&'a Polygon<f64>> for GGeometry<'b> {
    type Error = Error;

    fn try_from(other: &'a Polygon<f64>) -> Result<GGeometry<'b>, Self::Error> {
        let ring = LineRing(other.exterior());
        let geom_exterior: GGeometry = ring.try_into()?;

        let interiors: Vec<_> = other
            .interiors()
            .iter()
            .map(|i| LineRing(i).try_into())
            .collect::<Result<Vec<_>, _>>()?;

        GGeometry::create_polygon(geom_exterior, interiors)
    }
}

impl<'a> TryFrom<Polygon<f64>> for GGeometry<'a> {
    type Error = Error;

    fn try_from(other: Polygon<f64>) -> Result<GGeometry<'a>, Self::Error> {
        GGeometry::try_from(&other)
    }
}

impl<'a, 'b> TryFrom<&'a MultiPolygon<f64>> for GGeometry<'b> {
    type Error = Error;

    fn try_from(other: &'a MultiPolygon<f64>) -> Result<GGeometry<'b>, Self::Error> {
        let polygons: Vec<_> = other
            .0
            .iter()
            .map(|p| p.try_into())
            .collect::<Result<Vec<_>, _>>()?;

        GGeometry::create_multipolygon(polygons)
    }
}

impl<'a> TryFrom<MultiPolygon<f64>> for GGeometry<'a> {
    type Error = Error;

    fn try_from(other: MultiPolygon<f64>) -> Result<GGeometry<'a>, Self::Error> {
        GGeometry::try_from(&other)
    }
}

#[cfg(test)]
mod test {
    use super::LineRing;
    use crate::{Geom, Geometry as GGeometry};
    use geo_types::{Coordinate, LineString, MultiPolygon, Polygon};
    use std::convert::TryInto;

    fn coords(tuples: Vec<(f64, f64)>) -> Vec<Coordinate<f64>> {
        tuples.into_iter().map(Coordinate::from).collect()
    }

    #[test]
    fn polygon_contains_test() {
        let exterior = LineString(coords(vec![
            (0., 0.),
            (0., 1.),
            (1., 1.),
            (1., 0.),
            (0., 0.),
        ]));
        let interiors = vec![LineString(coords(vec![
            (0.1, 0.1),
            (0.1, 0.9),
            (0.9, 0.9),
            (0.9, 0.1),
            (0.1, 0.1),
        ]))];
        let p = Polygon::new(exterior.clone(), interiors.clone());

        assert_eq!(p.exterior(), &exterior);
        assert_eq!(p.interiors(), interiors.as_slice());

        let geom: GGeometry = p.try_into().unwrap();

        assert!(geom.contains(&geom).unwrap());

        let tmp: GGeometry = exterior.try_into().unwrap();

        assert!(!geom.contains(&tmp).unwrap());
        assert!(geom.covers(&tmp).unwrap());
        assert!(geom.touches(&tmp).unwrap());
    }

    #[test]
    fn multipolygon_contains_test() {
        let exterior = LineString(coords(vec![
            (0., 0.),
            (0., 1.),
            (1., 1.),
            (1., 0.),
            (0., 0.),
        ]));
        let interiors = vec![LineString(coords(vec![
            (0.1, 0.1),
            (0.1, 0.9),
            (0.9, 0.9),
            (0.9, 0.1),
            (0.1, 0.1),
        ]))];
        let p = Polygon::new(exterior, interiors);
        let mp = MultiPolygon(vec![p.clone()]);

        let geom: GGeometry = (&mp).try_into().unwrap();

        assert!(geom.contains(&geom).unwrap());
        assert!(geom
            .contains::<GGeometry>(&(&p).try_into().unwrap())
            .unwrap());
    }

    #[test]
    fn incorrect_multipolygon_test() {
        let exterior = LineString(coords(vec![(0., 0.)]));
        let interiors = vec![];
        let p = Polygon::new(exterior, interiors);
        let mp = MultiPolygon(vec![p.clone()]);

        let geom: Result<GGeometry, _> = mp.try_into();

        assert!(geom.is_err());
    }

    #[test]
    fn incorrect_polygon_not_closed() {
        // even if the polygon is not closed we can convert it to geos (we close it)
        let exterior = LineString(coords(vec![
            (0., 0.),
            (0., 2.),
            (2., 2.),
            (2., 0.),
            (0., 0.),
        ]));
        let interiors = vec![LineString(coords(vec![
            (0., 0.),
            (0., 1.),
            (1., 1.),
            (1., 0.),
            (0., 10.),
        ]))];
        let p = Polygon::new(exterior, interiors);
        let mp = MultiPolygon(vec![p]);

        let _g: GGeometry = mp.try_into().unwrap(); // no error
    }

    /// a linear ring can be empty
    #[test]
    fn empty_linear_ring() {
        let ls = LineString(vec![]);
        let geom: GGeometry = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().size().unwrap(), 0);
    }

    /// a linear ring should have at least 3 elements
    #[test]
    fn one_elt_linear_ring() {
        let ls = LineString(coords(vec![(0., 0.)]));
        let geom: Result<GGeometry, _> = LineRing(&ls).try_into();
        let error = geom.err().unwrap();
        assert_eq!(format!("{}", error), "Invalid geometry, impossible to create a LinearRing, A LinearRing must have at least 3 coordinates".to_string());
    }

    /// a linear ring should have at least 3 elements
    #[test]
    fn two_elt_linear_ring() {
        let ls = LineString(coords(vec![(0., 0.), (0., 1.)]));
        let geom: Result<GGeometry, _> = LineRing(&ls).try_into();
        let error = geom.err().unwrap();
        assert_eq!(format!("{}", error), "Invalid geometry, impossible to create a LinearRing, A LinearRing must have at least 3 coordinates".to_string());
    }

    /// an unclosed linearring is valid since we close it before giving it to geos
    #[test]
    fn unclosed_linear_ring() {
        let ls = LineString(coords(vec![(0., 0.), (0., 1.), (1., 2.)]));
        let geom: GGeometry = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().size().unwrap(), 4);
    }

    /// a bit tricky
    /// a ring should have at least 3 points.
    /// in the case of a closed ring with only element eg:
    ///
    /// let's take a point list: [p1, p2, p1]
    ///
    /// p1 ----- p2
    ///  ^-------|
    ///
    /// we consider it like a 3 points not closed ring (with the 2 last elements being equals...)
    ///
    /// shapely (the python geos wrapper) considers that too
    #[test]
    fn closed_2_points_linear_ring() {
        let ls = LineString(coords(vec![(0., 0.), (0., 1.), (1., 1.)]));
        let geom: GGeometry = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().expect("is_ring failed"));
        assert_eq!(geom.get_coord_seq().unwrap().size().unwrap(), 4);
    }

    /// a linear ring can be empty
    #[test]
    fn good_linear_ring() {
        let ls = LineString(coords(vec![(0., 0.), (0., 1.), (1., 2.), (0., 0.)]));
        let geom: GGeometry = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().size().unwrap(), 4);
    }
}
