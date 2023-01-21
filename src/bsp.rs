use glam::*;
use slotmap::new_key_type;
use slotmap::SlotMap;

#[derive(Debug, Clone)]
pub struct Plane {
    pub normal: Vec2,
    pub distance: f32,
}
impl Plane {
    pub fn distance_to_point(&self, point: Vec2) -> f32 {
        point.dot(self.normal) - self.distance
    }

    pub fn line_intersection(&self, (start, end): (Vec2, Vec2)) -> Option<Vec2> {
        let start_dist = self.distance_to_point(start);
        let end_dist = self.distance_to_point(end);

        if (start_dist > 0.0) == (end_dist > 0.0) {
            // both on the same side!
            println!("{:?} {:?} {:?} {:?}", start_dist, end_dist, start, end);
            return None;
        }

        let total_dist = end_dist - start_dist;

        let frac = start_dist.abs() / total_dist.abs();

        Some(start + (end - start) * frac)
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    // wound clockwise
    pub vertices: Vec<Vec2>,
}
impl Polygon {
    pub fn new_rect(min: Vec2, max: Vec2) -> Polygon {
        Polygon {
            vertices: vec![min, Vec2::new(min.x, max.y), max, Vec2::new(max.x, min.y)],
        }
    }
    pub fn clip_against_plane(&self, plane: &Plane, clipside_is_greater: bool) -> Polygon {
        if self.vertices.len() == 0 {
            return self.clone();
        }

        assert!(self.vertices.len() >= 3);

        // we may cross this plane many times,
        // though always an even number (else bug!)

        // what we really care about are edges between those on the "keep side"
        // and the "clipped side".
        // in this code, "clipside" means the side that will be clipped out
        let cmp_to_clipside = |a: f32, b: f32| {
            use std::cmp::Ordering;

            match a.partial_cmp(&b).unwrap() {
                Ordering::Less | Ordering::Equal => !clipside_is_greater,
                Ordering::Greater => clipside_is_greater,
            }
        };

        // this is an overapproximation but whatever
        let mut new_vertices = Vec::with_capacity(self.vertices.len());

        let mut prev = *self.vertices.last().unwrap();
        let mut prev_clipside = cmp_to_clipside(plane.distance_to_point(prev), 0.0);
        for &current in &self.vertices {
            let current_clipside = cmp_to_clipside(plane.distance_to_point(current), 0.0);

            match (prev_clipside, current_clipside) {
                (false, false) => {
                    // both on the keep side
                    new_vertices.push(current);
                }
                (false, true) => {
                    // we just entered the clip side!

                    // instead of emitting current, we must clip the edge between prev and current
                    let clipvert = plane.line_intersection((prev, current)).unwrap();
                    new_vertices.push(clipvert);
                }
                (true, false) => {
                    // we just left the clip side!
                    // we will emit current, but we must also emit a vertex from the clipped edge

                    let clipvert = plane.line_intersection((prev, current)).unwrap();
                    new_vertices.push(clipvert);

                    new_vertices.push(current);
                }
                (true, true) => {
                    // both on the clip side. emit no vertices.
                    ()
                }
            }
            prev = current;
            prev_clipside = current_clipside;
        }

        Polygon {
            vertices: new_vertices,
        }
    }
}

pub struct Bsp<T> {
    nodes: SlotMap<BspKey, BspNode<T>>,
    root: BspKey,
}
impl<T> Bsp<T> {
    pub fn new(root_val: T) -> Bsp<T> {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(BspNode::Leaf(BspLeaf(root_val)));

        Bsp { nodes, root }
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn root_key(&self) -> BspKey {
        self.root.clone()
    }

    pub fn leaf_index_for_point(&self, point: glam::Vec2) -> BspKey {
        let mut node = self.root_key();

        loop {
            match &self.nodes[node] {
                BspNode::Inode(inode) => {
                    let dist = inode.plane.distance_to_point(point);
                    if dist <= 0.0 {
                        node = inode.le;
                    } else {
                        node = inode.gt;
                    }
                }
                BspNode::Leaf(_) => {
                    return node;
                }
            }
        }
    }
    pub fn get_at_point(&self, point: Vec2) -> &T {
        match &self.nodes[self.leaf_index_for_point(point)] {
            BspNode::Inode(_) => unreachable!(),
            BspNode::Leaf(l) => &l.0,
        }
    }
    pub fn get_at_point_mut(&mut self, point: Vec2) -> &mut T {
        let i = self.leaf_index_for_point(point);
        match &mut self.nodes[i] {
            BspNode::Inode(_) => unreachable!(),
            BspNode::Leaf(l) => &mut l.0,
        }
    }
    pub fn split_at_point(&mut self, point: Vec2, normal: Vec2, new_val: T)
    where
        T: Clone,
    {
        debug_assert!(normal.is_normalized());

        let index = self.leaf_index_for_point(point);
        let plane = Plane {
            normal,
            distance: point.dot(normal),
        };

        self.nodes[index] = BspNode::Inode(BspInode {
            plane,
            le: self.nodes.insert(self.nodes[index].clone()),
            gt: self.nodes.insert(BspNode::Leaf(BspLeaf(new_val))),
        });
    }
    pub fn visit_leaf_polygons<F>(&self, start: BspKey, clip: Polygon, cb: &mut F)
    where
        F: FnMut(&BspLeaf<T>, &Polygon),
    {
        match &self.nodes[start] {
            BspNode::Inode(inode) => {
                let clipped_le = clip.clip_against_plane(&inode.plane, true);
                let clipped_gt = clip.clip_against_plane(&inode.plane, false);

                self.visit_leaf_polygons(inode.le, clipped_le, cb);
                self.visit_leaf_polygons(inode.gt, clipped_gt, cb);
            }
            BspNode::Leaf(l) => {
                cb(l, &clip);
            }
        }
    }
}

new_key_type! { pub struct BspKey; }

#[derive(Debug, Clone)]
pub struct BspInode {
    pub plane: Plane,

    pub le: BspKey,
    pub gt: BspKey,
}

#[derive(Debug, Clone)]
pub struct BspLeaf<T>(pub T);

#[derive(Debug, Clone)]
pub enum BspNode<T> {
    Inode(BspInode),
    Leaf(BspLeaf<T>),
}
