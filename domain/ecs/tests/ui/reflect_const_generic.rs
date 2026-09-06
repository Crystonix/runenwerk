use ecs::Reflect;

struct ConstField<const N: usize>([u8; N]);

impl<const N: usize> Reflect for ConstField<N> {
    fn type_info() -> ecs::TypeInfo {
        ecs::TypeInfo::new(
            std::any::type_name::<Self>(),
            "ConstField",
            ecs::ReflectShape::Opaque,
        )
    }
}

#[derive(Reflect)]
struct ConstGeneric<const N: usize> {
    value: ConstField<N>,
}

fn main() {
    let _ = ConstGeneric::<4>::type_info();
    let _ = ConstGeneric::<8>::type_info();
}
