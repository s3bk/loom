use image::{GrayImage, Luma};

pub enum Thickness {
    Hair,
    Width(f32)
}

fn darken(px: &mut Luma8, v: u8) {
    *px = *px.saturating_sub(v);
}

fn sample(img: &mut GrayImage, s: &Sampler, target: Rect, scale: (Real, Real) ) {
    let ref mut rng = rand::thread_rng();
    
    for n in 0 .. samples {
        // p is in domain space
        let p = c.sample(rng);
        
        // random offset
        //let noise = Vector2::new(uniform01.ind_sample(rng), uniform01.ind_sample(rng));
        // q is in canvas space
        let q: Vector2<N> = (p - offset).to_vector() * canvas_scale; //+ noise;
        
        // cast into pixel coordinates
        let sx: u32 = match cast(q.x) {
            Some(n) => n,
            None => continue
        };
        let sy: u32 = match cast(q.y) {
            Some(n) => n,
            None => continue
        };
        
        // if q is in the canvas ...
        if sx >= 0 && sx < subpixel_width && sy >= 0 && sy < subpixel_height {
            darken(img.get_pixel_mut(sx, sy), 16);
        }
    }
}
