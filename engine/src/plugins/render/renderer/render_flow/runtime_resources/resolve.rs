use super::*;
use runen_gpu::GpuWorkResourceId;

enum ResolvedColorTarget<'a> {
    Surface,
    Texture(&'a RuntimeTextureResource),
}

impl FlowRuntimeResources {
    pub(crate) fn resolve_resource_key_from_input(
        &self,
        value: &str,
    ) -> Option<RuntimeResourceKey> {
        if value == SURFACE_COLOR_RESOURCE_LABEL {
            return Some(RuntimeResourceKey::SurfaceColor);
        }
        if value == SURFACE_DEPTH_RESOURCE_LABEL {
            return Some(RuntimeResourceKey::SurfaceDepth);
        }

        if let Some(id) = self.resource_ids_by_label.get(value) {
            if let Some(RenderResourceDeclaration::TargetAlias(alias)) = self.descriptors.get(id)
                && let Some(binding) = self.target_alias_bindings.get(alias.binding_key())
            {
                return Some(runtime_resource_key_for_target_binding(binding));
            }
            return Some(RuntimeResourceKey::FlowOwned(*id));
        }

        None
    }

    fn kind_of_key(&self, key: &RuntimeResourceKey) -> Option<RuntimeResourceKind> {
        match key {
            RuntimeResourceKey::FlowOwned(id) => self.kinds.get(id).copied(),
            RuntimeResourceKey::InvocationUniform { .. } => Some(RuntimeResourceKind::BufferLike),
            RuntimeResourceKey::InvocationHistory { .. } => Some(RuntimeResourceKind::TextureLike),
            RuntimeResourceKey::DynamicTexture(_) => Some(RuntimeResourceKind::TextureLike),
            RuntimeResourceKey::SurfaceColor => Some(RuntimeResourceKind::TextureLike),
            RuntimeResourceKey::SurfaceDepth => None,
        }
    }

    fn descriptor_for_key(&self, key: &RuntimeResourceKey) -> Option<&RenderResourceDeclaration> {
        match key {
            RuntimeResourceKey::FlowOwned(id) => self.descriptors.get(id),
            RuntimeResourceKey::InvocationUniform { resource_id, .. } => {
                self.descriptors.get(resource_id)
            }
            RuntimeResourceKey::InvocationHistory { resource_id, .. } => {
                self.descriptors.get(resource_id)
            }
            RuntimeResourceKey::SurfaceColor
            | RuntimeResourceKey::SurfaceDepth
            | RuntimeResourceKey::DynamicTexture(_) => None,
        }
    }

    #[cfg(test)]
    pub fn kind_of(&self, id: &str) -> Option<RuntimeResourceKind> {
        self.resolve_resource_key_from_input(id)
            .and_then(|key| self.kind_of_key(&key))
    }

    pub fn kind_of_resource(
        &self,
        resource_key: RuntimeResourceKey,
    ) -> Option<RuntimeResourceKind> {
        self.kind_of_key(&resource_key)
    }

    pub fn capture_texture_class(
        &self,
        resource_id: &str,
        fallback_class: CaptureTextureClass,
    ) -> CaptureTextureClass {
        let Some(resource_key) = self.resolve_resource_key_from_input(resource_id) else {
            return fallback_class;
        };

        match resource_key {
            RuntimeResourceKey::SurfaceColor | RuntimeResourceKey::SurfaceDepth => {
                CaptureTextureClass::ImportedTexture
            }
            RuntimeResourceKey::DynamicTexture(_)
            | RuntimeResourceKey::InvocationUniform { .. } => fallback_class,
            RuntimeResourceKey::FlowOwned(_) | RuntimeResourceKey::InvocationHistory { .. } => {
                let Some(descriptor) = self.descriptor_for_key(&resource_key) else {
                    return fallback_class;
                };
                match descriptor {
                    RenderResourceDeclaration::DepthAttachment(_) => {
                        CaptureTextureClass::DepthTarget
                    }
                    RenderResourceDeclaration::History(_) => CaptureTextureClass::HistoryTexture,
                    RenderResourceDeclaration::ImportedTexture(_) => {
                        CaptureTextureClass::ImportedTexture
                    }
                    RenderResourceDeclaration::Sampled(_)
                    | RenderResourceDeclaration::StorageImage(_)
                    | RenderResourceDeclaration::ColorAttachment(_) => {
                        CaptureTextureClass::ColorTarget
                    }
                    RenderResourceDeclaration::TargetAlias(_) => {
                        CaptureTextureClass::ImportedTexture
                    }
                    RenderResourceDeclaration::Uniform(_)
                    | RenderResourceDeclaration::Storage(_)
                    | RenderResourceDeclaration::ImportedBuffer(_) => fallback_class,
                }
            }
        }
    }

    pub fn resolve_resource_key(
        &self,
        pass_id: RenderPassId,
        resource: &CompiledResourceRef,
        _role: &str,
    ) -> Result<RuntimeResourceKey> {
        match resource {
            CompiledResourceRef::FlowOwned(id) | CompiledResourceRef::Imported(id) => {
                if matches!(
                    self.descriptors.get(id),
                    Some(RenderResourceDeclaration::History(_))
                ) && let Some(invocation_id) = self.active_invocation_uniform_scope.as_ref()
                {
                    return Ok(RuntimeResourceKey::InvocationHistory {
                        invocation_id: invocation_id.clone(),
                        resource_id: *id,
                    });
                }
                Ok(RuntimeResourceKey::FlowOwned(*id))
            }
            CompiledResourceRef::TargetAlias(alias) => {
                let binding = self
                    .target_alias_bindings
                    .get(&alias.binding_key)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "pass '{}' references unbound target alias '{}'",
                            pass_id,
                            alias.binding_key
                        )
                    })?;
                match binding {
                    PreparedTargetBinding::DynamicTexture(key) => {
                        Ok(RuntimeResourceKey::DynamicTexture(key.clone()))
                    }
                    PreparedTargetBinding::SurfaceColor => Ok(RuntimeResourceKey::SurfaceColor),
                    PreparedTargetBinding::SurfaceDepth => Ok(RuntimeResourceKey::SurfaceDepth),
                    PreparedTargetBinding::FlowOwned(id) => {
                        if matches!(
                            self.descriptors.get(id),
                            Some(RenderResourceDeclaration::History(_))
                        ) && let Some(invocation_id) =
                            self.active_invocation_uniform_scope.as_ref()
                        {
                            return Ok(RuntimeResourceKey::InvocationHistory {
                                invocation_id: invocation_id.clone(),
                                resource_id: *id,
                            });
                        }
                        Ok(RuntimeResourceKey::FlowOwned(*id))
                    }
                }
            }
            CompiledResourceRef::ImportedBuiltin(CompiledBuiltinImport::SurfaceColor) => {
                Ok(RuntimeResourceKey::SurfaceColor)
            }
            CompiledResourceRef::ImportedBuiltin(CompiledBuiltinImport::SurfaceDepth) => {
                bail!(
                    "pass '{}' references imported builtin resource '{}' but surface-depth imports are not available in runtime execution yet; use flow-owned depth targets",
                    pass_id,
                    SURFACE_DEPTH_RESOURCE_LABEL
                )
            }
        }
    }

    fn resolve_color_target<'a>(
        &'a self,
        pass_id: RenderPassId,
        targets: &CompiledTargetPlan,
    ) -> Result<ResolvedColorTarget<'a>> {
        if targets.color_outputs.len() != 1 {
            bail!(
                "pass '{}' declares {} color outputs, but runtime execution currently requires exactly one color output",
                pass_id,
                targets.color_outputs.len()
            );
        }

        let output = targets.color_outputs.first().ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' is missing a color output target in execution plan",
                pass_id
            )
        })?;
        let output_key = self.resolve_resource_key(pass_id, output, "color_output")?;

        if output_key == RuntimeResourceKey::SurfaceColor {
            return Ok(ResolvedColorTarget::Surface);
        }

        let kind = self.kind_of_key(&output_key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' writes unknown color target '{}' during runtime encoding",
                pass_id,
                output_key
            )
        })?;
        if matches!(kind, RuntimeResourceKind::BufferLike) {
            bail!(
                "pass '{}' color target '{}' is buffer-like",
                pass_id,
                output_key
            );
        }

        let Some(texture) = self.texture_resource_for_key(&output_key) else {
            bail!(
                "pass '{}' targets imported texture '{}', but only '{}' is currently supported as imported color target",
                pass_id,
                output_key,
                SURFACE_COLOR_RESOURCE_LABEL
            );
        };
        if texture.is_depth {
            bail!(
                "pass '{}' color target '{}' is depth-only",
                pass_id,
                output_key
            );
        }

        Ok(ResolvedColorTarget::Texture(texture))
    }

    pub fn resolve_color_target_format_from_plan(
        &self,
        pass_id: RenderPassId,
        targets: &CompiledTargetPlan,
        surface_format: GpuTextureFormat,
    ) -> Result<GpuTextureFormat> {
        Ok(match self.resolve_color_target(pass_id, targets)? {
            ResolvedColorTarget::Surface => surface_format,
            ResolvedColorTarget::Texture(texture) => texture.format,
        })
    }

    fn resolve_depth_target<'a>(
        &'a self,
        pass_id: RenderPassId,
        targets: &CompiledTargetPlan,
    ) -> Result<Option<&'a RuntimeTextureResource>> {
        let Some(depth_target) = targets.depth_output.as_ref() else {
            return Ok(None);
        };
        let resource_key = self.resolve_resource_key(pass_id, depth_target, "depth_output")?;
        if resource_key == RuntimeResourceKey::SurfaceColor {
            bail!(
                "graphics pass '{}' uses '{}' as depth target, which is not supported",
                pass_id,
                SURFACE_COLOR_RESOURCE_LABEL
            );
        }

        let kind = self.kind_of_key(&resource_key).ok_or_else(|| {
            anyhow::anyhow!(
                "graphics pass '{}' uses unknown depth target '{}'",
                pass_id,
                resource_key
            )
        })?;
        if matches!(kind, RuntimeResourceKind::BufferLike) {
            bail!(
                "graphics pass '{}' uses '{}' as depth target, but it is buffer-like",
                pass_id,
                resource_key
            );
        }

        let Some(texture) = self.texture_resource_for_key(&resource_key) else {
            bail!(
                "graphics pass '{}' uses imported depth target '{}' but runtime currently supports only flow-owned depth targets",
                pass_id,
                resource_key
            );
        };
        if !texture.is_depth {
            bail!(
                "graphics pass '{}' uses '{}' as depth target, but it is not depth-capable",
                pass_id,
                resource_key
            );
        }

        Ok(Some(texture))
    }

    pub fn resolve_depth_target_format_from_plan(
        &self,
        pass_id: RenderPassId,
        targets: &CompiledTargetPlan,
    ) -> Result<Option<GpuTextureFormat>> {
        Ok(self
            .resolve_depth_target(pass_id, targets)?
            .map(|texture| texture.format))
    }

    pub fn resolve_logical_texture_binding(
        &self,
        pass_id: RenderPassId,
        resource_key: RuntimeResourceKey,
    ) -> Result<(&GpuTextureViewHandle, GpuTextureFormat, bool)> {
        if resource_key == RuntimeResourceKey::SurfaceColor {
            bail!(
                "pass '{}' binds '{}' as a shader texture before the exact G7A acquired logical view exists",
                pass_id,
                SURFACE_COLOR_RESOURCE_LABEL
            );
        }
        if matches!(resource_key, RuntimeResourceKey::DynamicTexture(_)) {
            bail!(
                "pass '{}' references dynamic texture '{}' through flow runtime resources; dynamic textures must be resolved through the renderer dynamic target cache",
                pass_id,
                resource_key
            );
        }

        let kind = self.kind_of_key(&resource_key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' references unknown resource '{}' during logical binding resolution",
                pass_id,
                resource_key
            )
        })?;
        if matches!(kind, RuntimeResourceKind::BufferLike) {
            bail!(
                "pass '{}' references '{}' as a texture, but it is buffer-like",
                pass_id,
                resource_key
            );
        }

        let Some(texture) = self.texture_resource_for_key(&resource_key) else {
            bail!(
                "pass '{}' references imported texture '{}' but only imported '{}' is supported in core runtime execution",
                pass_id,
                resource_key,
                SURFACE_COLOR_RESOURCE_LABEL
            );
        };
        Ok((&texture.view_handle, texture.format, texture.is_depth))
    }

    pub fn resolve_storage_buffer_ref<'a>(
        &'a self,
        pass_id: RenderPassId,
        resource: &CompiledResourceRef,
    ) -> Result<ResolvedBufferRef<'a>> {
        let resource_key = self.resolve_resource_key(pass_id, resource, "storage_buffer")?;
        self.resolve_storage_buffer_for_pass_by_key(pass_id, resource_key)
    }

    pub fn resolve_texture_from_label_without_surface<'a>(
        &'a self,
        pass_label: &str,
        resource_id: &str,
    ) -> Result<ResolvedTextureRef<'a>> {
        let resource_key = self
            .resolve_resource_key_from_input(resource_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "pass '{}' references unknown resource '{}' during logical texture resolution",
                    pass_label,
                    resource_id
                )
            })?;
        if resource_key == RuntimeResourceKey::SurfaceColor {
            bail!(
                "pass '{}' requires the exact acquired '{}' texture",
                pass_label,
                SURFACE_COLOR_RESOURCE_LABEL
            );
        }
        if matches!(resource_key, RuntimeResourceKey::DynamicTexture(_)) {
            bail!(
                "pass '{}' references dynamic texture '{}' through flow runtime resources; dynamic textures must be resolved through the renderer dynamic target cache",
                pass_label,
                resource_key
            );
        }
        let kind = self.kind_of_key(&resource_key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' references unknown resource '{}' during logical texture resolution",
                pass_label,
                resource_key
            )
        })?;
        if matches!(kind, RuntimeResourceKind::BufferLike) {
            bail!(
                "pass '{}' references '{}' as a texture, but it is buffer-like",
                pass_label,
                resource_key
            );
        }
        let texture = self
            .texture_resource_for_key(&resource_key)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "pass '{}' references unsupported imported texture '{}'",
                    pass_label,
                    resource_key
                )
            })?;
        Ok(ResolvedTextureRef {
            id: resource_key,
            view_handle: Some(&texture.view_handle),
            format: texture.format,
            size: texture.size,
            is_depth: texture.is_depth,
        })
    }

    fn texture_resource_for_key(
        &self,
        key: &RuntimeResourceKey,
    ) -> Option<&RuntimeTextureResource> {
        match key {
            RuntimeResourceKey::FlowOwned(resource_id) => self.textures.get(resource_id),
            RuntimeResourceKey::InvocationHistory {
                invocation_id,
                resource_id,
            } => self
                .invocation_history_textures
                .get(&(invocation_id.clone(), *resource_id)),
            _ => None,
        }
    }

    pub fn resolve_buffer_key<'a>(
        &'a self,
        pass_id: RenderPassId,
        resource_key: RuntimeResourceKey,
    ) -> Result<ResolvedBufferRef<'a>> {
        let kind = self.kind_of_key(&resource_key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' references unknown resource '{}' during runtime encoding",
                pass_id,
                resource_key
            )
        })?;
        if matches!(kind, RuntimeResourceKind::TextureLike) {
            bail!(
                "pass '{}' references '{}' as a buffer, but it is texture-like",
                pass_id,
                resource_key
            );
        }

        let RuntimeResourceKey::FlowOwned(resource_id) = &resource_key else {
            bail!(
                "pass '{}' references imported buffer '{}' but core runtime execution only supports flow-owned buffers",
                pass_id,
                resource_key
            );
        };

        let Some(buffer) = self.buffers.get(resource_id) else {
            bail!(
                "pass '{}' references imported buffer '{}' but core runtime execution only supports flow-owned buffers",
                pass_id,
                resource_key
            );
        };

        Ok(ResolvedBufferRef {
            id: resource_key,
            handle: &buffer.handle,
            size: buffer.size,
            kind: buffer.kind,
        })
    }

    pub fn resolve_uniform_buffer_for_pass<'a>(
        &'a self,
        pass_id: RenderPassId,
        resource_id: GpuWorkResourceId,
    ) -> Result<ResolvedBufferRef<'a>> {
        if let Some(scope) = self.active_invocation_uniform_scope.as_ref()
            && let Some(buffer) = self
                .invocation_uniform_buffers
                .get(&(scope.clone(), resource_id))
        {
            if !matches!(buffer.kind, RuntimeBufferKind::Uniform) {
                bail!(
                    "pass '{}' binds invocation-scoped '{}' as a uniform buffer but the resource is not uniform",
                    pass_id,
                    resource_id
                );
            }
            return Ok(ResolvedBufferRef {
                id: RuntimeResourceKey::InvocationUniform {
                    invocation_id: scope.clone(),
                    resource_id,
                },
                handle: &buffer.handle,
                size: buffer.size,
                kind: buffer.kind,
            });
        }

        let resolved =
            self.resolve_buffer_key(pass_id, RuntimeResourceKey::FlowOwned(resource_id))?;
        if !matches!(resolved.kind, RuntimeBufferKind::Uniform) {
            bail!(
                "pass '{}' binds '{}' as a uniform buffer but the resource is not uniform",
                pass_id,
                resolved.id
            );
        }
        Ok(resolved)
    }

    pub fn resolve_storage_buffer_for_pass_by_key<'a>(
        &'a self,
        pass_id: RenderPassId,
        resource_key: RuntimeResourceKey,
    ) -> Result<ResolvedBufferRef<'a>> {
        let resolved = self.resolve_buffer_key(pass_id, resource_key)?;
        if !matches!(resolved.kind, RuntimeBufferKind::Storage) {
            bail!(
                "pass '{}' binds '{}' as a storage buffer but the resource is not storage",
                pass_id,
                resolved.id
            );
        }
        Ok(resolved)
    }
}

fn runtime_resource_key_for_target_binding(binding: &PreparedTargetBinding) -> RuntimeResourceKey {
    match binding {
        PreparedTargetBinding::DynamicTexture(key) => {
            RuntimeResourceKey::DynamicTexture(key.clone())
        }
        PreparedTargetBinding::SurfaceColor => RuntimeResourceKey::SurfaceColor,
        PreparedTargetBinding::SurfaceDepth => RuntimeResourceKey::SurfaceDepth,
        PreparedTargetBinding::FlowOwned(id) => RuntimeResourceKey::FlowOwned(*id),
    }
}
