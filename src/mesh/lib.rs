//#![crate_name = "mesh"]
//mod mesh;

use std::cmp::*;
use std::collections::HashMap;
use std::f32::NAN;
use std::fmt;
use std::old_io::{BufferedReader,Reader};
use std::hash::{Hash, Hasher};
use std::mem::{transmute};
use std::vec::Vec;

#[derive(PartialEq, PartialOrd, Copy)] //Show,
pub struct Vector3D {
    x: f32,
    y: f32,
    z: f32,
}

// PartialEq is derived above, but we need to impl Eq since floats don't get
// it by default. This function is just a
impl Eq for Vector3D {
    #[inline(always)]
    fn assert_receiver_is_total_eq(&self) {}
}

impl fmt::Debug for Vector3D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

impl Vector3D {
    fn read(r: &mut Reader) -> Vector3D {
        let xr = r.read_le_f32();
        let yr = r.read_le_f32();
        let zr = r.read_le_f32();
        if xr.is_ok() && yr.is_ok() && zr.is_ok() {
            let v = Vector3D { x: xr.ok().unwrap(), y: yr.ok().unwrap(), z: zr.ok().unwrap() };
            return v
        }
        return Vector3D { x: NAN, y: NAN, z: NAN };
    }
}

// Implement Hash since there is no default for f32. We'll just hash the bits
// since we know the f32s will all be canonical from reading.
impl Hash for Vector3D {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        //println!("V3D hash {:p}", self);
        let x: u32 = unsafe { transmute(self.x) };
        x.hash(state);
        let y: u32 = unsafe { transmute(self.y) };
        y.hash(state);
        let z: u32 = unsafe { transmute(self.z) };
        z.hash(state);
        //println!("V3D hash done {:p}", self);
    }
}

#[derive(PartialEq, Eq, Hash, Copy)]
pub struct StlFacet {
    n : Vector3D,
    v1: Vector3D,
    v2: Vector3D,
    v3: Vector3D,
    abc: u16,
}

impl fmt::Debug for StlFacet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{:?}]({:?}-{:?}-{:?})[{:X}]",
            &self.n, &self.v1, &self.v2, &self.v3, self.abc)
    }
}

impl StlFacet {
    fn read(r: &mut Reader) -> StlFacet {
        let n = Vector3D::read(r);
        let v1 = Vector3D::read(r);
        let v2 = Vector3D::read(r);
        let v3 = Vector3D::read(r);
        let abc = r.read_le_u16().unwrap();
        StlFacet { n:n, v1:v1, v2:v2, v3:v3, abc:abc }
    }
}

pub struct VertexMap {
    vertices: HashMap<Vector3D,usize>,
}

impl VertexMap {
    pub fn new() -> VertexMap {
        VertexMap {
            vertices: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn get(&self, v3d: &Vector3D) -> usize {
        *self.vertices.get(v3d).unwrap()
    }

    pub fn add(&mut self, vertex: Vector3D) -> usize {
        if self.vertices.contains_key(&vertex) {
            //println!("add existing {}", vector3d_str(&vertex));
            *self.vertices.get(&vertex).unwrap()
        } else {
            //println!("add new {}", vector3d_str(&vertex));
            let idx = self.vertices.len();
            self.vertices.insert(vertex, idx);
            idx
        }
    }

    pub fn vector(&self) -> Vec<Vector3D> {
        let mut v: Vec<Vector3D> = Vec::with_capacity(self.vertices.len());
        for (v3d, idx) in self.vertices.iter() {
            v.push(*v3d);  // Funkiness just to get v expanded. Below writes the real values.
        }
        for (v3d, idx) in self.vertices.iter() {
            //println!("v3d:{:?} idx:{}", v3d, idx);
            v[*idx] = *v3d;
        }
        //for i in 0..v.len() {
        //    println!("v3d[{}]: {:?}", i, v[i]);
        //}
        v
    }
}

#[derive(PartialEq, Debug, Eq, Hash, Copy)]
pub struct Facet {
    v1: usize,
    v2: usize,
    v3: usize,
}

pub struct Mesh {
    vertices: Vec<Vector3D>,
    facets: Vec<Facet>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh {
            vertices: Vec::new(),
            facets: Vec::new(),
        }
    }

    fn indexed_vertices(fv: &Vec<StlFacet>, vm: &VertexMap) -> Vec<Facet> {
        let mut v: Vec<Facet> = Vec::with_capacity(fv.len());
        for f in fv.iter() {
            let v1 = vm.get(&f.v1);
            let v2 = vm.get(&f.v2);
            let v3 = vm.get(&f.v3);
            v.push(Facet{
                v1: v1,
                v2: v2,
                v3: v3,
            })
        }
        v
    }

    pub fn new_from_stl(fv: &Vec<StlFacet>, vm: &VertexMap) -> Mesh {
        let vs = vm.vector();
        let fs = Mesh::indexed_vertices(fv, vm);
        Mesh {
            vertices: vs,
            facets: fs,
        }
    }

    pub fn read(r: &mut BufferedReader<Reader>) -> Mesh {

        let buf = [0; 80];
        let header = match r.read(&buf) {
            Ok(nread) => {
                let hdr = String::from_utf8(&buf).unwrap();
                println!("Header: \"{}\"", hdr);
                hdr
            }
            Err(e) => {
                println!("Truncated file: {}", e);
                return Mesh{vertices: Vec::new(), facets: Vec::new()};
            }
        };

        return if header.starts_with("solid ") {
            println!("Is ASCII STL");
            Mesh::read_ascii(r)
        }
        else {
            println!("Is binary STL");
            Mesh::read_binary(r)
        }
    }

    fn read_ascii(r: &mut BufferedReader<Reader>) -> Mesh {
        //solid vcg
        //  facet normal 7.733874e-001 -3.151335e-002 6.331499e-001
        //    outer loop
        //      vertex  2.137833e+001 5.385936e+001 1.350253e+001
        //      vertex  2.341920e+001 4.700068e+001 1.066826e+001
        //      vertex  2.859041e+001 5.985682e+001 4.991545e+000
        //    endloop
        //  endfacet
        //endsolid vcg
        for line in r.lines() {
            print!("{}", line.unwrap());
        }
        Mesh::new() // TODO
    }

    fn read_binary(r: &mut Reader) -> Mesh {
        let facet_count = match r.read_le_u32() {
            Ok(c) => { println!("Facets: {}", c); c},
            Err(e) => { println!("Truncated file: {}", e); 0},
        };

        let mut facets: Vec<StlFacet> = Vec::with_capacity(facet_count as usize);
        let mut vertices = VertexMap::new();
        println!("Collections ready");

        for fi in range(0, facet_count) {
            let f = StlFacet::read(r);
            let v1i = vertices.add(f.v1);
            let v2i = vertices.add(f.v2);
            let v3i = vertices.add(f.v3);
            println!("  Facet[{}]: {:?} => {}-{}-{}", fi, f, v1i, v2i, v3i);
            facets.push(f);
        }
        println!("Vertices: {}", vertices.len());

        Mesh::new_from_stl(&facets, &vertices)
    }
}
