use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
};

// Bootloader (фича map_physical_memory) отображает всю физическую память
// в виртуальную начиная с адреса physical_memory_offset. Поэтому физический
// адрес X доступен по виртуальному physical_memory_offset + X, и так ядро
// читает и меняет таблицы страниц, которые лежат в физической памяти

/// Создаёт OffsetPageTable для активной таблицы страниц 4-го уровня.
///
/// unsafe: вызывающий гарантирует, что вся физическая память отображена
/// по `physical_memory_offset`. Вызывать один раз, иначе появятся две
/// `&mut` ссылки на одну таблицу
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// Возвращает ссылку на активную таблицу 4-го уровня, адрес которой лежит в CR3
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// Переводит виртуальный адрес в физический, вручную проходя 4 уровня
/// таблиц страниц. Возвращает `None`, если адрес не отображён.
///
/// Это учебная версия: то же самое делает `Translate::translate_addr`
/// у `OffsetPageTable`. На huge pages (например, в области
/// `physical_memory_offset`) эта версия паникует, а `OffsetPageTable` - нет.
///
/// unsafe: те же требования, что у `init`
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

// Безопасная функция внутри unsafe: так область unsafe ограничена
// одним разыменованием указателя на таблицу
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::page_table::FrameError;

    let (level_4_table_frame, _) = Cr3::read();
    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    // на каждом уровне запись таблицы указывает на фрейм таблицы следующего
    // уровня, а на последнем - на фрейм с самими данными
    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()))
}

/// Пример из статьи: отображает `page` на фрейм VGA-буфера (0xb8000)
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    // unsafe: фрейм VGA-буфера уже используется через WRITER, и теперь к нему
    // будет второй доступ. Для примера это допустимо, в общем случае - нет
    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_to_result.expect("map_to failed").flush();
}

/// Выдаёт свободные фреймы из карты памяти, которую передал bootloader.
/// Освобождать фреймы пока нельзя
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    /// номер следующего фрейма, который будет выдан
    next: usize,
}

impl BootInfoFrameAllocator {
    /// unsafe: вызывающий гарантирует, что фреймы, помеченные в карте как
    /// `Usable`, действительно ничем не заняты
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Все свободные фреймы по порядку: регионы `Usable`, нарезанные по 4 KiB
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

// unsafe impl: реализация обещает, что не выдаст один фрейм дважды
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // итератор каждый раз создаётся заново, поэтому выдача фрейма - O(n).
        // Для примера хватает, позже аллокатор стоит заменить
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
