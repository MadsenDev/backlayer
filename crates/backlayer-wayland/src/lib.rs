use std::ptr::NonNull;

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use serde::{Deserialize, Serialize};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
};
use thiserror::Error;
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BootstrapStatus {
    Uninitialized,
    Planned {
        protocol: &'static str,
        input_passthrough: bool,
    },
    Ready {
        protocol: &'static str,
        input_passthrough: bool,
        output_count: usize,
        configured: bool,
        bound_output: Option<String>,
    },
}

#[derive(Debug, Error)]
pub enum WaylandError {
    #[error("failed to connect to wayland compositor: {0}")]
    Connect(#[from] wayland_client::ConnectError),
    #[error("failed to initialize wayland registry: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),
    #[error("required wayland global is missing: {0}")]
    Bind(#[from] wayland_client::globals::BindError),
    #[error("failed to dispatch wayland event queue: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),
    #[error("failed to create wgpu surface: {0}")]
    Surface(String),
}

#[derive(Debug, Default, Clone)]
pub struct LayerShellRuntime;

impl LayerShellRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn bootstrap_status(&self) -> BootstrapStatus {
        BootstrapStatus::Planned {
            protocol: "zwlr_layer_shell_v1",
            input_passthrough: true,
        }
    }

    pub fn probe(&self) -> Result<BootstrapStatus, WaylandError> {
        self.probe_on_output(None)
    }

    pub fn probe_on_output(
        &self,
        output_name: Option<&str>,
    ) -> Result<BootstrapStatus, WaylandError> {
        Ok(self.start_session_on_output(output_name)?.status())
    }

    pub fn start_session_on_output(
        &self,
        output_name: Option<&str>,
    ) -> Result<LayerSurfaceSession, WaylandError> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut event_queue) = registry_queue_init(&conn)?;
        let qh = event_queue.handle();

        let compositor_state = CompositorState::bind(&globals, &qh)?;
        let layer_shell = LayerShell::bind(&globals, &qh)?;
        let output_state = OutputState::new(&globals, &qh);
        let registry_state = RegistryState::new(&globals);

        let mut probe = LayerShellProbe {
            registry_state,
            output_state,
            compositor_state,
            layer_surface: None,
            bound_output: None,
            width: 1,
            height: 1,
            configured: false,
            closed: false,
        };

        event_queue.roundtrip(&mut probe)?;

        let selected_output = match output_name {
            Some(name) => probe.find_output(name),
            None => None,
        };

        let surface = probe.compositor_state.create_surface(&qh);
        let layer_surface = layer_shell.create_layer_surface(
            &qh,
            surface,
            Layer::Background,
            Some("backlayer"),
            selected_output.as_ref(),
        );
        layer_surface.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.set_size(0, 0);
        layer_surface.commit();
        probe.layer_surface = Some(layer_surface);

        probe.bound_output = selected_output
            .as_ref()
            .and_then(|output| probe.output_state.info(output))
            .and_then(|info| info.name);

        event_queue.roundtrip(&mut probe)?;

        Ok(LayerSurfaceSession {
            conn,
            event_queue,
            state: probe,
        })
    }
}

pub struct LayerSurfaceSession {
    conn: Connection,
    event_queue: EventQueue<LayerShellProbe>,
    state: LayerShellProbe,
}

impl LayerSurfaceSession {
    pub fn status(&self) -> BootstrapStatus {
        BootstrapStatus::Ready {
            protocol: "zwlr_layer_shell_v1",
            input_passthrough: true,
            output_count: self.state.output_state.outputs().count(),
            configured: self.state.configured && !self.state.closed,
            bound_output: self.state.bound_output.clone(),
        }
    }

    pub fn dispatch_pending(&mut self) -> Result<BootstrapStatus, WaylandError> {
        self.event_queue.dispatch_pending(&mut self.state)?;
        Ok(self.status())
    }

    pub fn blocking_dispatch(&mut self) -> Result<BootstrapStatus, WaylandError> {
        self.event_queue.blocking_dispatch(&mut self.state)?;
        Ok(self.status())
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.state.width.max(1), self.state.height.max(1))
    }

    pub unsafe fn create_wgpu_surface(
        &self,
        instance: &wgpu::Instance,
    ) -> Result<wgpu::Surface<'static>, WaylandError> {
        let layer_surface = self
            .state
            .layer_surface
            .as_ref()
            .expect("layer surface should exist while session is active");
        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(self.conn.backend().display_ptr() as *mut _).expect("display ptr"),
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(layer_surface.wl_surface().id().as_ptr() as *mut _).expect("surface ptr"),
        ));

        unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle,
                raw_window_handle,
            })
        }
        .map_err(|error| WaylandError::Surface(error.to_string()))
    }
}

struct LayerShellProbe {
    registry_state: RegistryState,
    output_state: OutputState,
    #[allow(dead_code)]
    compositor_state: CompositorState,
    layer_surface: Option<LayerSurface>,
    bound_output: Option<String>,
    width: u32,
    height: u32,
    configured: bool,
    closed: bool,
}

impl LayerShellProbe {
    fn find_output(&self, output_name: &str) -> Option<wl_output::WlOutput> {
        self.output_state.outputs().find(|output| {
            self.output_state
                .info(output)
                .and_then(|info| info.name)
                .as_deref()
                == Some(output_name)
        })
    }
}

impl CompositorHandler for LayerShellProbe {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for LayerShellProbe {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for LayerShellProbe {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.closed = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if self
            .layer_surface
            .as_ref()
            .is_some_and(|active| active.wl_surface() == layer.wl_surface())
        {
            self.width = _configure.new_size.0.max(1);
            self.height = _configure.new_size.1.max(1);
            self.configured = true;
        }
    }
}

delegate_compositor!(LayerShellProbe);
delegate_output!(LayerShellProbe);
delegate_layer!(LayerShellProbe);
delegate_registry!(LayerShellProbe);

impl ProvidesRegistryState for LayerShellProbe {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}
