use super::{Vertex, Shader, Shadable, Drawable, Surface};

use std::collections::HashMap;
use std::os::raw::c_void;

use gl;
use gl::types::*;
use gl_raii::{GLVertexBuffer, GLVertex, BufferUsage,
              VertexAttribData, GLSLType, GLPrim,
              GLVertexArray, GLIndexBuffer, GLBuffer,
              GLProgram, GLShader, ShaderType, BufModder};

use cgmath::Matrix3;
use cgmath::prelude::*;

static mut ID_COUNTER: u64 = 0;

pub struct BufferData {
    id: u64,
    verts: GLVertexBuffer<Vertex>,
    vert_indices: GLIndexBuffer,
    verts_vao: GLVertexArray<Vertex>
}

impl BufferData {
    pub fn new() -> BufferData {
        let id = unsafe{ ID_COUNTER };
        unsafe{ ID_COUNTER += 1 };

        let verts = GLVertexBuffer::new(0, BufferUsage::Static);
        let vert_indices = GLIndexBuffer::new(0, BufferUsage::Static);
        let verts_vao = GLVertexArray::new(&verts, Some(&vert_indices));
        BufferData {
            id: id,
            verts: verts,
            vert_indices: vert_indices,
            verts_vao: verts_vao
        }
    }
}

unsafe impl GLVertex for Vertex {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData] {
        const VAD: &'static [VertexAttribData] = &[
            // Relative point
            VertexAttribData {
                index: 0,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 0
            },

            // Absolute point
            VertexAttribData {
                index: 1,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 8
            },

            // Normal
            VertexAttribData {
                index: 2,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 16
            },

            // Color
            VertexAttribData {
                index: 3,
                glsl_type: GLSLType::Vec4(GLPrim::NUByte),
                offset: 24
            }
        ];

        VAD
    }
}

struct ColorVertexProgram {
    program: GLProgram,
    transform_matrix_uniform: GLint
}

impl ColorVertexProgram {
    fn new() -> ColorVertexProgram {
        use std::fs::File;
        use std::io::Read;

        let mut colored_vertex_string = String::new();
        File::open("./src/shaders/colored_vertex.vert").unwrap().read_to_string(&mut colored_vertex_string).unwrap();
        let colored_vertex_vert = GLShader::new(ShaderType::Vertex, &colored_vertex_string).unwrap();

        let mut color_passthrough_string = String::new();
        File::open("./src/shaders/color_passthrough.frag").unwrap().read_to_string(&mut color_passthrough_string).unwrap();
        let color_passthrough_frag = GLShader::new(ShaderType::Fragment, &color_passthrough_string).unwrap();

        let program = GLProgram::new(&colored_vertex_vert, &color_passthrough_frag).unwrap();

        let transform_matrix_uniform = unsafe{ gl::GetUniformLocation(program.handle, "transform_matrix\0".as_ptr() as *const GLchar) };

        ColorVertexProgram {
            program: program,
            transform_matrix_uniform: transform_matrix_uniform
        }
    }
}

struct IDMapEntry {
    num_updates: u64,
    base_vertex_vec: Vec<BaseVertexData>
}

pub struct Facade {
    id_map: HashMap<u64, IDMapEntry>,
    color_passthrough: ColorVertexProgram
}

impl Facade {
    pub fn new<F: Fn(&str) -> *const c_void>(load_with: F) -> Facade {
        gl::load_with(load_with);

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        Facade {
            id_map: HashMap::with_capacity(32),
            color_passthrough: ColorVertexProgram::new()
        }
    }

    pub fn resize(&self, x: u32, y: u32) {
        unsafe{ gl::Viewport(0, 0, x as GLint, y as GLint) };
    }

    pub fn surface<'a>(&'a mut self) -> GLSurface {
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        GLSurface {
            facade: self
        }
    }
}

pub struct GLSurface<'a> {
    facade: &'a mut Facade
}

impl<'a> Surface for GLSurface<'a> {
    fn draw<D: Drawable>(&mut self, drawable: &D) {
        use std::collections::hash_map::Entry;

        let buffers = drawable.buffer_data();
        // Whether or not to re-upload any data to the GPU buffers
        let update_buffers: bool;

        let id_map_entry: &mut IDMapEntry;
        match self.facade.id_map.entry(buffers.id) {
            Entry::Occupied(mut entry) => {
                update_buffers = !drawable.num_updates() == entry.get().num_updates;
                entry.get_mut().num_updates = drawable.num_updates();
                id_map_entry = entry.into_mut();
            }
            Entry::Vacant(entry)   => {
                update_buffers = true;
                id_map_entry = entry.insert(
                    IDMapEntry {
                        num_updates: drawable.num_updates(),
                        base_vertex_vec: Vec::new()
                    }
                );
            }
        }
        
        if update_buffers {
            buffers.verts.with(|vert_modder|
                buffers.vert_indices.with(|index_modder| {
                    let mut bud = BufferUpdateData {
                        vert_offset: 0,
                        vert_modder: vert_modder,
                        index_offset: 0,
                        offsetted_indices: Vec::new(),
                        index_modder: index_modder,

                        base_vertex_vec: vec![Default::default(); 1],
                        matrix_stack: vec![One::one(); 1]
                    };

                    bud.update_buffers(drawable);

                    // If `drawable` is a composite, then there will be one extra BaseVertexData pushed to the vector
                    // that we need to get rid of. This does that.
                    if let Shader::Composite{..} = drawable.shader_data() {
                        bud.base_vertex_vec.pop();
                    }
                    id_map_entry.base_vertex_vec = bud.base_vertex_vec;
                })
            );
        }

        let transform_matrix_uniform = self.facade.color_passthrough.transform_matrix_uniform;
        self.facade.color_passthrough.program.with(|_| {
            buffers.verts_vao.with(|_| {
                for bvd in id_map_entry.base_vertex_vec.iter() {unsafe{
                    gl::UniformMatrix3fv(
                        transform_matrix_uniform,
                        1,
                        gl::FALSE,
                        bvd.matrix.as_ptr()
                    );

                    gl::DrawElementsBaseVertex(
                        gl::TRIANGLES, 
                        bvd.count as GLsizei, 
                        gl::UNSIGNED_SHORT, 
                        bvd.offset as *const _,
                        bvd.base_vertex as GLint
                    );
                }}
            });
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct BaseVertexData {
    offset: usize,
    count: usize,
    base_vertex: usize,
    matrix: Matrix3<f32>
}

impl Default for BaseVertexData {
    fn default() -> BaseVertexData {
        BaseVertexData {
            offset: 0,
            count: 0,
            base_vertex: 0,
            matrix: One::one()
        }
    }
}

struct BufferUpdateData<'a> {
    vert_offset: usize,
    vert_modder: BufModder<'a, Vertex, GLVertexBuffer<Vertex>>,
    index_offset: usize,
    offsetted_indices: Vec<u16>,
    index_modder: BufModder<'a, u16, GLIndexBuffer>,

    base_vertex_vec: Vec<BaseVertexData>,
    matrix_stack: Vec<Matrix3<f32>>
}

impl<'a> BufferUpdateData<'a> {
    fn update_buffers<S: Shadable>(&mut self, shadable: &S) {
        match shadable.shader_data() {
            Shader::Verts {verts, indices} => {
                self.vert_modder.sub_data(self.vert_offset, verts);

                let bvd = self.base_vertex_vec.last_mut().unwrap();
                bvd.count += indices.len();

                if self.index_offset > 0 {
                    // Clear the vector without de-allocating memory
                    unsafe{ self.offsetted_indices.set_len(0) };

                    // Because every vertex is being stored in one vertex buffer, we need to offset the indices so that
                    // they all get drawn properly
                    let vert_offset = self.vert_offset as u16;
                    self.offsetted_indices.extend(indices.iter().map(|i| *i + vert_offset));

                    self.index_modder.sub_data(self.index_offset, &self.offsetted_indices);
                } else {
                    self.index_modder.sub_data(self.index_offset, indices);
                }

                self.vert_offset += verts.len();
                self.index_offset += indices.len();
            }

            Shader::Composite{foreground, fill, backdrop, rect, ..} => {
                let center = rect.center().to_rel();
                let height = rect.upleft.rel.y - rect.lowright.rel.y;
                let width = rect.upleft.rel.x - rect.lowright.rel.x;

                let new_matrix = self.matrix_stack.last().unwrap() * Matrix3::new(
                    width/2.0,        0.0, 0.0,
                          0.0, height/2.0, 0.0,
                     center.x,   center.y, 1.0
                );

                self.base_vertex_vec.last_mut().unwrap().matrix = new_matrix;
                self.matrix_stack.push(new_matrix);

                // We order the vertices so that OpenGL draws them back-to-front
                self.update_buffers(&backdrop);
                self.update_buffers(&fill);
                self.update_buffers(&foreground);

                self.base_vertex_vec.push(Default::default());
            }

            Shader::None => ()
        }
    }
}
