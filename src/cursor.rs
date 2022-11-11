use triangles::model::cmodel::{Face, Model};

// next char block: LURD
#[allow(dead_code)]
pub fn draw1([x1, _y1, _x2, y2]: [f32; 4], ssize: [u32; 2]) -> Model {
	let vs = vec![
		[x1, 0.0, 0.0, 1.0],
		[x1 + 1.0, 0.0, 0.0, 1.0],
		[x1, ssize[1] as f32, 0.0, 1.0],
		[x1 + 1.0, ssize[1] as f32, 0.0, 1.0],
		[0.0, y2, 0.0, 1.0],
		[0.0, y2 + 1.0, 0.0, 1.0],
		[ssize[0] as f32, y2, 0.0, 1.0],
		[ssize[0] as f32, y2 + 1.0, 0.0, 1.0],
	];
	let faces = vec![
		Face {
			vid: [0, 1, 3],
			color: [1.0, 0.0, 0.0, 1.0],
			uvid: [0; 3],
			layer: -1,
		},
		Face {
			vid: [0, 2, 3],
			color: [1.0, 0.0, 0.0, 1.0],
			uvid: [0; 3],
			layer: -1,
		},
		Face {
			vid: [4, 5, 7],
			color: [1.0, 0.0, 0.0, 1.0],
			uvid: [0; 3],
			layer: -1,
		},
		Face {
			vid: [4, 6, 7],
			color: [1.0, 0.0, 0.0, 1.0],
			uvid: [0; 3],
			layer: -1,
		},
	];
	Model {
		vs,
		uvs: Vec::new(),
		faces,
	}
}

#[allow(dead_code)]
pub fn draw2([x1, y1, _x2, y2]: [f32; 4], _ssize: [u32; 2]) -> Model {
	let vs = vec![
		[x1, y1, 0.0, 1.0],
		[x1, y2, 0.0, 1.0],
		[x1 + 1.0, y1, 0.0, 1.0],
		[x1 + 1.0, y2, 0.0, 1.0],
	];
	let faces = vec![
		Face {
			vid: [0, 1, 2],
			color: [1.0; 4],
			uvid: [0; 3],
			layer: -1,
		},
		Face {
			vid: [3, 1, 2],
			color: [1.0; 4],
			uvid: [0; 3],
			layer: -1,
		},
	];
	Model {
		vs,
		uvs: Vec::new(),
		faces,
	}
}
