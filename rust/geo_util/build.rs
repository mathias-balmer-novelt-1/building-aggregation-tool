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
fn main() {
    if cfg!(windows) {
		 println!(r"cargo:rustc-link-search=C:\Program Files\PostgreSQL\11\lib");
		println!(r"cargo:rustc-link-search=D:\PostgreSQL\12\lib");
		println!(r"cargo:rustc-link-search=C:\OSGeo4W64\bin");
		println!(r"cargo:rustc-link-search=C:\OSGeo4W64\lib");
        println!("cargo:rustc-env=GDAL_HOME={}", r"C:\OSGeo4W64");
        println!(r"cargo:rustc-link-search=C:\OSGeo4W64\bin");
        println!(r"cargo:rustc-link-search=C:\OSGeo4W64\lib");
    }
}