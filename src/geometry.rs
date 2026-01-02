use crate::blockchain::Sha256Hash;
use crate::crypto::Address;
use fixed::types::I32F32;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Coordinate type for deterministic geometric calculations.
pub type Coord = I32F32;
/// Tolerance for fixed-point comparisons to check for degeneracy/equality.
pub const GEOMETRIC_TOLERANCE: Coord = I32F32::from_bits(42950); // Approx 1e-5

// ----------------------------------------------------------------------------
// 1.4 Coordinate System: Point
// ----------------------------------------------------------------------------

/// Represents a 2D point with deterministic fixed-point coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

impl Point {
    /// Maximum allowed coordinate value to prevent overflow/precision issues
    pub const MAX_COORDINATE: Coord = I32F32::from_bits(i64::MAX);

    /// Creates a new Point.
    #[inline]
    pub fn new(x: Coord, y: Coord) -> Self {
        Point { x, y }
    }

    /// Validates that the point has finite coordinates within reasonable bounds
    pub fn is_valid(&self) -> bool {
        self.x < Self::MAX_COORDINATE && self.y < Self::MAX_COORDINATE
    }

    /// Calculates the midpoint between this point and another.
    #[inline]
    pub fn midpoint(&self, other: &Point) -> Point {
        Point::new((self.x + other.x) / 2, (self.y + other.y) / 2)
    }

    /// Calculates a simple cryptographic hash of the point data.
    #[inline]
    pub fn hash(&self) -> Sha256Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.x.to_le_bytes());
        hasher.update(self.y.to_le_bytes());
        hasher.finalize().into()
    }

    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    /// Checks for exact equality with another point.
    pub fn equals(&self, other: &Point) -> bool {
        self.x == other.x && self.y == other.y
    }
}

// ----------------------------------------------------------------------------
// 1.3 Triangle Data Structure & Core Methods
// ----------------------------------------------------------------------------

/// Represents a triangle defined by three points (vertices).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Triangle {
    pub a: Point,
    pub b: Point,
    pub c: Point,
    pub parent_hash: Option<Sha256Hash>,
    pub owner: Address,
    /// Effective value of this triangle.
    #[serde(default)]
    pub value: Option<Coord>,
}

impl Triangle {
    /// Creates a new Triangle from three vertices.
    pub fn new(
        a: Point,
        b: Point,
        c: Point,
        parent_hash: Option<Sha256Hash>,
        owner: Address,
    ) -> Self {
        Triangle {
            a,
            b,
            c,
            parent_hash,
            owner,
            value: None,
        }
    }

    /// Creates a new Triangle with an explicit value.
    pub fn new_with_value(
        a: Point,
        b: Point,
        c: Point,
        parent_hash: Option<Sha256Hash>,
        owner: Address,
        value: Coord,
    ) -> Self {
        Triangle {
            a,
            b,
            c,
            parent_hash,
            owner,
            value: Some(value),
        }
    }

    /// Returns the effective value of this triangle.
    pub fn effective_value(&self) -> Coord {
        self.value.unwrap_or_else(|| self.area())
    }

    /// Creates a new triangle with a different owner.
    pub fn change_owner(&self, new_owner: Address) -> Self {
        let mut new_triangle = self.clone();
        new_triangle.owner = new_owner;
        new_triangle
    }

    /// Creates a new triangle with a specific effective value.
    pub fn with_effective_value(mut self, value: Coord) -> Self {
        self.value = Some(value);
        self
    }

    /// Calculates the area of the triangle using the Shoelace formula.
    pub fn area(&self) -> Coord {
        let val = (self.a.x * (self.b.y - self.c.y)
            + self.b.x * (self.c.y - self.a.y)
            + self.c.x * (self.a.y - self.b.y))
            .abs();
        val / 2
    }

    /// Calculates the unique cryptographic hash of the triangle.
    ///
    /// This method creates a canonical representation of the triangle by
    /// sorting its vertices' raw byte data, then hashing the combined result
    /// along with owner and value. This approach is more efficient than
    /// hashing each point individually and combining the results.
    pub fn hash(&self) -> Sha256Hash {
        let mut hasher = Sha256::new();

        // Create a canonical representation by sorting vertices
        let mut vertices = [self.a, self.b, self.c];
        vertices.sort_by(|p1, p2| {
            p1.x.to_bits()
                .cmp(&p2.x.to_bits())
                .then_with(|| p1.y.to_bits().cmp(&p2.y.to_bits()))
        });

        // Hash the sorted vertices' raw bytes
        for vertex in &vertices {
            hasher.update(vertex.x.to_le_bytes());
            hasher.update(vertex.y.to_le_bytes());
        }

        // Include owner and value in the hash for uniqueness
        hasher.update(self.owner);
        if let Some(value) = self.value {
            hasher.update(value.to_le_bytes());
        }

        hasher.finalize().into()
    }

    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    // ------------------------------------------------------------------------
    // 1.6 Genesis Triangle Implementation
    // ------------------------------------------------------------------------

    /// Defines the canonical Genesis Triangle for the TrinityChain.
    pub fn genesis() -> Self {
        Triangle::new(
            Point::new(Coord::from_num(0), Coord::from_num(0)),
            Point::new(Coord::from_num(1.7320508), Coord::from_num(0)),
            Point::new(Coord::from_num(0.8660254), Coord::from_num(1.5)),
            None,
            [0; 32],
        )
    }

    // ------------------------------------------------------------------------
    // 1.7 Subdivision Algorithm
    // ------------------------------------------------------------------------

    /// Subdivides the triangle into three smaller triangles using midpoints,
    /// creating a Sierpiński-like structure. This method is geometrically
    /// stable and prevents the creation of degenerate (long, thin) triangles.
    ///
    /// Value is conserved by explicitly assigning one-third of the parent's
    /// effective value to each child, decoupling spendable value from raw
    /// geometric area.
    pub fn subdivide(&self) -> [Triangle; 3] {
        // Calculate midpoints of the triangle's sides
        let mid_ab = self.a.midpoint(&self.b);
        let mid_bc = self.b.midpoint(&self.c);
        let mid_ca = self.c.midpoint(&self.a);

        let parent_hash = Some(self.hash());
        let child_value = self.effective_value() / 3;

        // Create the three new triangles at the corners
        let t1 =
            Triangle::new_with_value(self.a, mid_ab, mid_ca, parent_hash, self.owner, child_value);
        let t2 =
            Triangle::new_with_value(mid_ab, self.b, mid_bc, parent_hash, self.owner, child_value);
        let t3 =
            Triangle::new_with_value(mid_ca, mid_bc, self.c, parent_hash, self.owner, child_value);

        [t1, t2, t3]
    }

    // ------------------------------------------------------------------------
    // 1.8 Geometric Validation
    // ------------------------------------------------------------------------

    /// Checks if the triangle is geometrically valid.
    pub fn is_valid(&self) -> bool {
        if !self.a.is_valid() || !self.b.is_valid() || !self.c.is_valid() {
            return false;
        }
        self.area() > GEOMETRIC_TOLERANCE
    }
}

// ----------------------------------------------------------------------------
// Testing
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_address(s: &str) -> Address {
        let mut address = [0u8; 32];
        let bytes = s.as_bytes();
        address[..bytes.len()].copy_from_slice(bytes);
        address
    }

    fn setup_test_triangle() -> Triangle {
        Triangle::new(
            Point::new(Coord::from_num(0), Coord::from_num(0)),
            Point::new(Coord::from_num(10), Coord::from_num(0)),
            Point::new(Coord::from_num(0), Coord::from_num(10)),
            None,
            create_test_address("test_owner"),
        )
    }

    #[test]
    fn test_point_midpoint() {
        let p1 = Point::new(Coord::from_num(1), Coord::from_num(1));
        let p2 = Point::new(Coord::from_num(5), Coord::from_num(5));
        let midpoint = p1.midpoint(&p2);
        assert_eq!(midpoint, Point::new(Coord::from_num(3), Coord::from_num(3)));
    }

    #[test]
    fn test_triangle_area() {
        let t = setup_test_triangle();
        assert_eq!(t.area(), Coord::from_num(50));
    }

    #[test]
    fn test_triangle_hash_is_canonical() {
        let p1 = Point::new(Coord::from_num(1), Coord::from_num(2));
        let p2 = Point::new(Coord::from_num(3), Coord::from_num(4));
        let p3 = Point::new(Coord::from_num(5), Coord::from_num(6));

        let t1 = Triangle::new(p1, p2, p3, None, create_test_address("owner1"));
        let t2 = Triangle::new(p3, p1, p2, None, create_test_address("owner1"));

        assert_eq!(t1.hash(), t2.hash());
    }

    #[test]
    fn test_triangle_hash_changes_with_owner() {
        let p1 = Point::new(Coord::from_num(1), Coord::from_num(2));
        let p2 = Point::new(Coord::from_num(3), Coord::from_num(4));
        let p3 = Point::new(Coord::from_num(5), Coord::from_num(6));

        let t1 = Triangle::new(p1, p2, p3, None, create_test_address("owner1"));
        let t2 = Triangle::new(p1, p2, p3, None, create_test_address("owner2"));

        assert_ne!(t1.hash(), t2.hash());
    }

    #[test]
    fn test_genesis_triangle_is_canonical() {
        let g1 = Triangle::genesis();
        let expected_area = Coord::from_num(1.2990381);
        assert!((g1.area() - expected_area).abs() < Coord::from_num(1e-6));
    }

    #[test]
    fn test_subdivision_correctness() {
        let parent = setup_test_triangle();
        let parent_value = parent.effective_value();
        let children = parent.subdivide();

        // The sum of the children's effective values must equal the parent's.
        let total_child_value: Coord = children.iter().map(|t| t.effective_value()).sum();
        assert!((total_child_value - parent_value).abs() < GEOMETRIC_TOLERANCE);

        // The geometric area of the children should be 75% of the parent's,
        // as the central triangle is removed in a Sierpiński subdivision.
        let total_child_area: Coord = children.iter().map(|t| t.area()).sum();
        let expected_area = parent.area() * Coord::from_num(0.75);
        assert!((total_child_area - expected_area).abs() < GEOMETRIC_TOLERANCE);
    }

    #[test]
    fn test_geometric_validation_valid() {
        let t = setup_test_triangle();
        assert!(t.is_valid());

        let g = Triangle::genesis();
        assert!(g.is_valid());
    }

    #[test]
    fn test_geometric_validation_invalid_degenerate() {
        let t_degenerate = Triangle::new(
            Point::new(Coord::from_num(1), Coord::from_num(1)),
            Point::new(Coord::from_num(2), Coord::from_num(2)),
            Point::new(Coord::from_num(3), Coord::from_num(3)),
            None,
            create_test_address("owner"),
        );
        assert!(!t_degenerate.is_valid());
    }
}
