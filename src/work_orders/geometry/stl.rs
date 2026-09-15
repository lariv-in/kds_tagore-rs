//! STL volume calculation using the `stl_io` crate.
//!
//! Calculates the exact volume of a 3D closed surface mesh using the divergence theorem:
//! V = 1/6 * sum( v1 . (v2 x v3) ) for all triangular facets.

use std::io::{Cursor, Read};

#[derive(Debug, thiserror::Error)]
pub enum StlError {
    #[error("I/O error reading STL: {0}")]
    Io(#[from] std::io::Error),
    #[error("Mesh is empty or contains no triangles")]
    EmptyMesh,
}

/// Calculate the volume of an STL mesh from raw bytes using `stl_io`.
pub fn calculate_stl_volume(data: &[u8]) -> Result<f64, StlError> {
    let mut cursor = Cursor::new(data);
    let mesh = stl_io::read_stl(&mut cursor)?;

    if mesh.faces.is_empty() || mesh.vertices.is_empty() {
        return Err(StlError::EmptyMesh);
    }

    let mut total_volume = 0.0;

    for face in &mesh.faces {
        let v1 = &mesh.vertices[face.vertices[0]];
        let v2 = &mesh.vertices[face.vertices[1]];
        let v3 = &mesh.vertices[face.vertices[2]];

        let (x1, y1, z1) = (v1[0] as f64, v1[1] as f64, v1[2] as f64);
        let (x2, y2, z2) = (v2[0] as f64, v2[1] as f64, v2[2] as f64);
        let (x3, y3, z3) = (v3[0] as f64, v3[1] as f64, v3[2] as f64);

        let cross_x = y2 * z3 - z2 * y3;
        let cross_y = z2 * x3 - x2 * z3;
        let cross_z = x2 * y3 - y2 * x3;

        let signed_tet_vol = (x1 * cross_x + y1 * cross_y + z1 * cross_z) / 6.0;
        total_volume += signed_tet_vol;
    }

    Ok(total_volume.abs())
}

/// Calculate the volume of an STL file directly from a reader using `stl_io`.
pub fn calculate_stl_reader_volume<R: Read + std::io::Seek>(reader: &mut R) -> Result<f64, StlError> {
    let mesh = stl_io::read_stl(reader)?;

    if mesh.faces.is_empty() || mesh.vertices.is_empty() {
        return Err(StlError::EmptyMesh);
    }

    let mut total_volume = 0.0;

    for face in &mesh.faces {
        let v1 = &mesh.vertices[face.vertices[0]];
        let v2 = &mesh.vertices[face.vertices[1]];
        let v3 = &mesh.vertices[face.vertices[2]];

        let (x1, y1, z1) = (v1[0] as f64, v1[1] as f64, v1[2] as f64);
        let (x2, y2, z2) = (v2[0] as f64, v2[1] as f64, v2[2] as f64);
        let (x3, y3, z3) = (v3[0] as f64, v3[1] as f64, v3[2] as f64);

        let cross_x = y2 * z3 - z2 * y3;
        let cross_y = z2 * x3 - x2 * z3;
        let cross_z = x2 * y3 - y2 * x3;

        let signed_tet_vol = (x1 * cross_x + y1 * cross_y + z1 * cross_z) / 6.0;
        total_volume += signed_tet_vol;
    }

    Ok(total_volume.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_volume_ascii() {
        let ascii_cube = r#"solid cube
facet normal 0 0 -1
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 1 1 0
  endloop
endfacet
facet normal 0 0 -1
  outer loop
    vertex 0 0 0
    vertex 1 1 0
    vertex 0 1 0
  endloop
endfacet
facet normal 0 0 1
  outer loop
    vertex 0 0 1
    vertex 1 1 1
    vertex 1 0 1
  endloop
endfacet
facet normal 0 0 1
  outer loop
    vertex 0 0 1
    vertex 0 1 1
    vertex 1 1 1
  endloop
endfacet
facet normal 0 -1 0
  outer loop
    vertex 0 0 0
    vertex 1 0 1
    vertex 1 0 0
  endloop
endfacet
facet normal 0 -1 0
  outer loop
    vertex 0 0 0
    vertex 0 0 1
    vertex 1 0 1
  endloop
endfacet
facet normal 0 1 0
  outer loop
    vertex 0 1 0
    vertex 1 1 0
    vertex 1 1 1
  endloop
endfacet
facet normal 0 1 0
  outer loop
    vertex 0 1 0
    vertex 1 1 1
    vertex 0 1 1
  endloop
endfacet
facet normal -1 0 0
  outer loop
    vertex 0 0 0
    vertex 0 1 0
    vertex 0 1 1
  endloop
endfacet
facet normal -1 0 0
  outer loop
    vertex 0 0 0
    vertex 0 1 1
    vertex 0 0 1
  endloop
endfacet
facet normal 1 0 0
  outer loop
    vertex 1 0 0
    vertex 1 1 1
    vertex 1 1 0
  endloop
endfacet
facet normal 1 0 0
  outer loop
    vertex 1 0 0
    vertex 1 0 1
    vertex 1 1 1
  endloop
endfacet
endsolid cube"#;

        let volume = calculate_stl_volume(ascii_cube.as_bytes()).expect("Valid volume");
        assert!((volume - 1.0).abs() < 1e-6, "Volume of 1x1x1 cube should be 1.0");
    }
}
