use glow::*;
use sdl2::event::Event;
use std::fs::read;
use std::env;
use std::process;
fn main() {
    let args: Vec<String> = env::args().collect();
    let path: &str = &args[1];

    let bitty = match read(path) {
        Ok(data) => data,
        Err(_) => {
            println!("Error: Could not read '{}'", path);
            process::exit(1);
        }
    };
    let bitty = bitty.as_slice();
    let resolution = (bitty.len() as f32).sqrt().round();
    unsafe {
        // OpenGL context creation, SDL2 window setup
        let (gl, shader_version, window, mut event_pump, _context) = {
            let sdl = sdl2::init().unwrap();
            let video = sdl.video().unwrap();
            let gl_attr = video.gl_attr();
            gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
            gl_attr.set_context_version(4, 0);
            let window = video
                .window("bitty", resolution as u32, resolution as u32)
                .opengl()
                .build()
                .unwrap();
            let context = window.gl_create_context().unwrap();
            let gl =
                glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _);
            let event_pump = sdl.event_pump().unwrap();
            (gl, "#version 430", window, event_pump, context)
        };

        let vao = gl
            .create_vertex_array()
            .unwrap();
        gl.bind_vertex_array(Some(vao));

        let program = gl.create_program().expect("Render error: failed to create program");

        let (vert_shader, frag_shader) = (
            r#"
            const vec2 verts[6] = vec2[6](
                vec2(-1.0f,  1.0f),
                vec2(-1.0f, -1.0f),
                vec2( 1.0f, -1.0f),
                vec2(-1.0f,  1.0f),
                vec2( 1.0f, -1.0f),
                vec2( 1.0f,  1.0f)
            );
            void main() {
                vec2 pos = verts[gl_VertexID];
                gl_Position = vec4(pos, 0.0, 1.0);
            }
            "#, 
            r#"
            layout(std430, binding = 0) buffer data {
                uint bitty[];
            };
            uniform float res;
            uniform vec4 mouse;

            out vec4 col;
            void main(){
                vec2 crd = vec2(gl_FragCoord.x, -gl_FragCoord.y + res);
                float cell = res*(crd.y-1.0) + crd.x; 
                int byte = int(mod(cell, 4.0));
                int i = int(floor(cell/4.0));
                uint value;
                if(byte == 0){
                    value = (bitty[i])&0xff;
                }
                else if(byte==1){
                    value = (bitty[i]>>8)&0xff;
                }
                else if(byte==2){
                    value = (bitty[i]>>16)&0xff;
                }
                else if(byte==3){
                    value = (bitty[i]>>24)&0xff;
                }
                if(mouse.z==1.0){
                    value = (uint((mouse.x/res)*255.)==value) ? 255: 0;
                }
                col = vec4(float(value)/255.);
            }
            "#,
        );

        let shader_sources = [
            (glow::VERTEX_SHADER, vert_shader),
            (glow::FRAGMENT_SHADER, frag_shader),
        ];

        let mut shaders = Vec::with_capacity(shader_sources.len());
        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .unwrap();
            gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!("Render error: failed to compile shader: {}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }

        let data = gl.create_buffer().unwrap();
        gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(data));
        gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, bitty,  STATIC_DRAW);
        gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(data));
        gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(data));


        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("Render error: failed to link program: {}", gl.get_program_info_log(program));
        }

        let loc_res = gl.get_uniform_location(program, "res");
        let loc_mouse = gl.get_uniform_location(program, "mouse");

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        gl.use_program(Some(program));
        gl.clear_color(1.0, 0.0, 1.0, 1.0);

        let mut running = true;
        let mut mouse : [f32; 4] = [0.5*resolution, 0.5*resolution, 0.0, 0.0];

        while running {
            {
                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. } => running = false,
                        Event::MouseButtonDown {mouse_btn, x, y, ..} => {
                            if let sdl2::mouse::MouseButton::Left = mouse_btn {
                                mouse = [x as f32, (-y as f32) + resolution, 1.0, mouse[3]];
                            };
                        },
                        Event::MouseButtonUp {mouse_btn, ..} => {
                            if let sdl2::mouse::MouseButton::Left = mouse_btn {
                                mouse[2] = 0.0;
                            };
                        },
                        Event::MouseMotion {mousestate, x, y, ..} => {
                            if mousestate.left() {
                                mouse = [x as f32, (-y as f32) + resolution, 1.0, mouse[3]];
                            }else{
                                mouse[2] = 0.0;
                            }
                        },
                        _ => {}
                    }
                }
            }

            gl.clear(COLOR_BUFFER_BIT);

            gl.uniform_1_f32(loc_res.as_ref(), resolution);
            gl.uniform_4_f32(loc_mouse.as_ref(), mouse[0], mouse[1], mouse[2], mouse[3]);

            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            window.gl_swap_window();

            if !running {
                gl.delete_program(program);
                gl.delete_vertex_array(vao);
            }
        }
    }
}
