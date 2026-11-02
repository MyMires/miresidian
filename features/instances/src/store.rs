//! 泛型对象容器；单实例能力始终存在，多实例后端由 runtime feature 编译。

use crate::InstanceMode;
use miresidian_codec::{MiresidianError, MiresidianResult};

enum Storage<T> {
    Single(Option<T>),
    #[cfg(feature = "runtime")]
    Multiple(Vec<T>),
}

/// 不依赖节点或传输实现的实例容器。
///
/// 单实例插入已有对象时返回原待插入值，由调用方负责清理；容器不会静默覆盖设备。
pub struct InstanceStore<T> {
    storage: Storage<T>,
}

impl<T> Default for InstanceStore<T> {
    fn default() -> Self {
        Self {
            storage: Storage::Single(None),
        }
    }
}

impl<T> InstanceStore<T> {
    /// 在空容器中配置数量策略；已登记对象时不能切换策略。
    pub fn set_mode(&mut self, mode: InstanceMode) -> MiresidianResult<()> {
        let same = match &self.storage {
            Storage::Single(_) => mode == InstanceMode::Single,
            #[cfg(feature = "runtime")]
            Storage::Multiple(_) => mode == InstanceMode::Multiple,
        };
        if same {
            return Ok(());
        }
        if !self.is_empty() {
            return Err(MiresidianError::InvalidArgument {
                field: "instances".to_owned(),
                message: "存在对象时不能切换实例数量策略".to_owned(),
            });
        }
        self.storage = match mode {
            InstanceMode::Single => Storage::Single(None),
            #[cfg(feature = "runtime")]
            InstanceMode::Multiple => Storage::Multiple(Vec::new()),
            #[cfg(not(feature = "runtime"))]
            InstanceMode::Multiple => {
                return Err(MiresidianError::NotSupported(
                    "实例 crate 未启用 runtime feature".to_owned(),
                ));
            }
        };
        Ok(())
    }

    /// 是否没有已登记对象。
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// 按登记顺序访问对象。
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match &self.storage {
            Storage::Single(value) => value.as_slice().iter(),
            #[cfg(feature = "runtime")]
            Storage::Multiple(values) => values.iter(),
        }
    }

    /// 登记对象；单实例槽已占用时原样返回待登记对象。
    pub fn push(&mut self, value: T) -> Result<(), T> {
        match &mut self.storage {
            Storage::Single(slot) if slot.is_none() => {
                *slot = Some(value);
                Ok(())
            }
            Storage::Single(_) => Err(value),
            #[cfg(feature = "runtime")]
            Storage::Multiple(values) => {
                values.push(value);
                Ok(())
            }
        }
    }

    /// 移出最后一个登记对象，生命周期清理由调用方执行。
    pub fn pop(&mut self) -> Option<T> {
        match &mut self.storage {
            Storage::Single(value) => value.take(),
            #[cfg(feature = "runtime")]
            Storage::Multiple(values) => values.pop(),
        }
    }

    /// 移出给定下标的对象。
    ///
    /// # Panics
    /// 下标必须由当前容器的迭代结果取得；越界时与 Vec::remove 一样 panic。
    pub fn remove(&mut self, index: usize) -> T {
        match &mut self.storage {
            Storage::Single(value) => {
                assert_eq!(index, 0);
                value.take().expect("对象下标必须存在")
            }
            #[cfg(feature = "runtime")]
            Storage::Multiple(values) => values.remove(index),
        }
    }
}
