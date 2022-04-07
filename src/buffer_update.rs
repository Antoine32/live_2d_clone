use bytemuck::Pod;
use num::{Integer, NumCast};
use specs::{
    prelude::ComponentEvent, shred::DynamicSystemData, BitSet, Component, FlaggedStorage, Read,
    ReadStorage, ReaderId, System, VecStorage, World, WorldExt, Write,
};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};

use crate::actor::Queue;

#[derive(Debug)]
pub struct DataIndex<T: Pod + Send + Sync + Default + Sized> {
    pub index: usize,
    pub array_index: usize,
    pub amount: usize,
    pub time_passed: Instant,
    pub waiting_time: u128,
    pub active: bool,
    phantom: PhantomData<T>,
}

impl<T: Pod + Send + Sync + Default + Sized> Default for DataIndex<T> {
    fn default() -> Self {
        todo!() // only need the trait because of a requirement to make it a ressource with specs, it wouln'd be possible to run the program without it being propely initialized anyway
    }
}

impl<T: Pod + Send + Sync + Default + Sized> DataIndex<T> {
    pub fn new(index: usize, array_index: usize, amount: usize, waiting_time: Duration) -> Self {
        DataIndex {
            index,
            array_index,
            amount,
            time_passed: Instant::now(),
            waiting_time: waiting_time.as_millis(),
            active: false,
            phantom: PhantomData,
        }
    }

    pub fn get_score(&self) -> u128 {
        let time_passed = self.time_passed.elapsed().as_millis();

        if self.active && time_passed > self.waiting_time {
            time_passed - self.waiting_time
        } else {
            0
        }
    }

    pub fn get_index<S: Sized + Integer + NumCast>(
        &self,
        data: &DataManager<T>,
        adjust: bool,
    ) -> S {
        let index = self.index
            + if self.active && adjust {
                data.idle_size
            } else {
                0
            };
        let index = NumCast::from(index).unwrap();

        index
    }

    pub fn get_array_index<S: Sized + Integer + NumCast>(
        &self,
        data: &DataManager<T>,
        adjust: bool,
    ) -> S {
        let array_index = self.array_index
            + if self.active && adjust {
                data.idle_data.len()
            } else {
                0
            };
        let array_index = NumCast::from(array_index).unwrap();

        array_index
    }

    pub fn get_range<S: Sized + Integer + NumCast + Copy>(
        &self,
        data: &DataManager<T>,
        adjust: bool,
    ) -> std::ops::Range<S> {
        let array_index = self.get_array_index(data, adjust);
        let last = array_index + NumCast::from(self.amount).unwrap();

        array_index..last
    }
}

#[derive(Debug, Default)]
pub struct ArcDataIndex<T: Pod + Send + Sync + Default + Sized>(pub Arc<Mutex<DataIndex<T>>>);

impl<T: Pod + Send + Sync + Default + Sized> Component for ArcDataIndex<T> {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl<T: Pod + Send + Sync + Default + Sized> ArcDataIndex<T> {
    pub fn new(index: usize, array_index: usize, amount: usize, waiting_time: Duration) -> Self {
        Self {
            0: Arc::new(Mutex::new(DataIndex::new(
                index,
                array_index,
                amount,
                waiting_time,
            ))),
        }
    }

    pub fn get_index<S: Sized + Integer + NumCast>(
        &self,
        data: &DataManager<T>,
        adjust: bool,
    ) -> S {
        self.0.lock().unwrap().get_index::<S>(data, adjust)
    }

    pub fn get_array_index<S: Sized + Integer + NumCast>(
        &self,
        data: &DataManager<T>,
        adjust: bool,
    ) -> S {
        self.0.lock().unwrap().get_array_index::<S>(data, adjust)
    }

    pub fn get_range<S: Sized + Integer + NumCast + Copy>(
        &self,
        data: &DataManager<T>,
        adjust: bool,
    ) -> std::ops::Range<S> {
        self.0.lock().unwrap().get_range::<S>(data, adjust)
    }
}

impl<T: Pod + Send + Sync + Default + Sized> Clone for ArcDataIndex<T> {
    fn clone(&self) -> Self {
        Self { 0: self.0.clone() }
    }
}

#[derive(Default, Debug)]
pub struct DataManager<T: Pod + Send + Sync + Default + Sized> {
    pub active_data: Vec<T>,
    pub idle_data: Vec<T>,
    pub indices: Vec<Weak<Mutex<DataIndex<T>>>>,
    pub idle_size: usize,
    pub active_size: usize,
}

impl<T: Pod + Send + Sync + Default + Sized> DataManager<T> {
    pub fn new(
        idle_data: Vec<T>,
        indices: Vec<Weak<Mutex<DataIndex<T>>>>,
        idle_size: usize,
    ) -> Self {
        Self {
            active_data: Vec::new(),
            idle_data,
            indices,
            idle_size,
            active_size: 0,
        }
    }

    pub fn get_mut_range(&mut self, index: &DataIndex<T>) -> &mut [T] {
        let range = index.get_range::<usize>(self, false);

        if index.active {
            &mut self.active_data[range]
        } else {
            &mut self.idle_data[range]
        }
    }

    pub fn get_mut_index(&mut self, index: &DataIndex<T>) -> &mut T {
        if index.active {
            &mut self.active_data[index.index]
        } else {
            &mut self.idle_data[index.index]
        }
    }
}

#[derive(Debug)]
pub struct DataBuffer<T: Pod + Send + Sync + Default + Sized + Sized> {
    pub buffer: wgpu::Buffer,
    phantom: PhantomData<T>,
}

impl<T: Pod + Send + Sync + Default + Sized + Sized> Default for DataBuffer<T> {
    fn default() -> Self {
        todo!() // only need the trait because of a requirement to make it a ressource with specs, it wouln'd be possible to run the program without it being propely initialized anyway
    }
}

impl<T: Pod + Send + Sync + Default + Sized + Sized> DataBuffer<T> {
    pub fn new(buffer: wgpu::Buffer) -> Self {
        Self {
            buffer,
            phantom: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct DataBufferUpdater<T: Pod + Send + Sync + Default + Sized> {
    pub dirty: BitSet,
    pub reader_id: Option<ReaderId<ComponentEvent>>,
    phantom: PhantomData<T>,
}

impl<T: Pod + Send + Sync + Default + Sized> DataBufferUpdater<T> {
    fn make_idle(index_buf: usize, data: &mut DataManager<T>) -> Option<()> {
        let last_range;
        let new_index = data.idle_size + data.active_size - 1;
        let amount;

        {
            let index = data.indices[index_buf].upgrade()?;
            let mut index = index.lock().unwrap();
            last_range = index.get_range::<usize>(&data, false);
            amount = index.amount;

            index.active = false;
            index.index = data.idle_size;
            index.array_index = data.idle_data.len();
        }

        let mut last_content: Vec<_> = data.active_data.drain(last_range).collect();
        data.idle_data.append(&mut last_content);

        let index = data.indices.remove(index_buf);
        let idle_len = data.idle_size;

        if idle_len < data.indices.len() {
            data.indices.insert(idle_len, index);
        } else {
            data.indices.push(index);
        }

        if index_buf != new_index {
            for i in index_buf..new_index {
                let other = &mut data.indices[i].upgrade()?;
                let mut other = other.lock().unwrap();
                other.index -= 1;
                other.array_index -= amount;
            }
        }

        data.idle_size += 1;
        data.active_size -= 1;

        Some(())
    }
}

impl<'a, T: Pod + Send + Sync + Default + Sized> System<'a> for DataBufferUpdater<T> {
    type SystemData = (
        ReadStorage<'a, ArcDataIndex<T>>,
        Write<'a, DataManager<T>>,
        Read<'a, DataBuffer<T>>,
        Read<'a, Queue>,
    );

    fn run(&mut self, (indices, mut data, buffer, queue): Self::SystemData) {
        use specs::Join;

        self.dirty.clear();

        let events = indices.channel().read(self.reader_id.as_mut().unwrap());

        for event in events {
            match event {
                ComponentEvent::Modified(id) | ComponentEvent::Inserted(id) => {
                    self.dirty.add(*id);
                }
                ComponentEvent::Removed(_) => (),
            }
        }

        let mut update = false;
        let mut update_idle: Option<usize> = None;

        {
            let mut updates = (&indices, &self.dirty)
                .join()
                .map(|(index, _)| {
                    (
                        index.get_index::<usize>(&data, true),
                        index.get_array_index::<usize>(&data, true),
                    )
                })
                .collect::<Vec<_>>();

            updates.sort_by(|(a, _), (b, _)| b.cmp(&a));

            //let mut to_fix = Vec::new();

            for (index_buf, array_index_buf) in updates.into_iter() {
                data.indices[index_buf]
                    .upgrade()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .time_passed = Instant::now();

                update = true;

                if !data.indices[index_buf]
                    .upgrade()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .active
                {
                    let last_range;
                    let new_index = data.idle_size - 1;
                    let amount;

                    {
                        let index = data.indices[index_buf].upgrade().unwrap();
                        let mut index = index.lock().unwrap();
                        last_range = index.get_range::<usize>(&data, false);
                        amount = index.amount;

                        index.active = true;
                        index.index = data.active_size;
                        index.array_index = data.active_data.len();
                    }

                    let mut last_content: Vec<_> = data.idle_data.drain(last_range).collect();
                    data.active_data.append(&mut last_content);

                    let index = data.indices.remove(index_buf);
                    data.indices.push(index);

                    if index_buf != new_index {
                        for i in index_buf..new_index {
                            let other = &mut data.indices[i].upgrade().unwrap();
                            let mut other = other.lock().unwrap();
                            other.index -= 1;
                            other.array_index -= amount;
                        }

                        update_idle = Some(array_index_buf);
                    }

                    data.idle_size -= 1;
                    data.active_size += 1;
                }
            }
        }

        if update {
            let mut potentials = Vec::new();
            let mut total_score = 0;

            let mut updates = (&indices, !&self.dirty)
                .join()
                .filter(|(index, _)| index.0.lock().unwrap().active)
                .map(|(index, _)| index.0.lock().unwrap().get_index::<usize>(&data, true))
                .collect::<Vec<_>>();

            updates.sort();

            for index_buf in updates.into_iter() {
                let index = data.indices[index_buf].upgrade().unwrap();
                let index = index.lock().unwrap();
                let score = index.get_score();

                if score > 0 {
                    total_score += score;
                    potentials.push(index_buf);
                }
            }

            if total_score >= 1000 {
                let index = data.idle_data.len();

                for (i, index_buf) in potentials.into_iter().rev().enumerate() {
                    Self::make_idle(index_buf + i, &mut data);
                }

                if update_idle.is_none() {
                    update_idle = Some(index);
                }
            }

            if let Some(at) = update_idle {
                if at < data.idle_data.len() {
                    let content = data.idle_data.split_at(at).1;

                    queue.0.write_buffer(
                        &buffer.buffer,
                        at as u64 * std::mem::size_of::<T>() as u64,
                        bytemuck::cast_slice(&content),
                    );
                }
            }

            queue.0.write_buffer(
                &buffer.buffer,
                data.idle_data.len() as u64 * std::mem::size_of::<T>() as u64,
                bytemuck::cast_slice(&data.active_data),
            );
        }
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
        self.reader_id = Some(world.write_component::<ArcDataIndex<T>>().register_reader());
    }
}
