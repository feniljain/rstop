use crate::logger::Logger;
use queue::Queue;
use sysinfo::{DiskExt, Processor, ProcessorExt, SystemExt};
use termion::event::Key;

#[derive(Clone, Debug)]
pub struct App {
    pub name: String,
    pub temps: Vec<Vec<String>>,
    pub disk_usage: Vec<Vec<String>>,
    pub should_quit: bool,
    pub cpu_usage_queue: Vec<Queue<(f64, f64)>>,
    pub cpu_usage_points: Vec<Vec<(f64, f64)>>,
    pub max_capacity_queue: usize,
    pub memory: Memory,
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub free_memory: u64,
    pub free_swap: u64,
    pub memory_queue: Queue<(f64, f64)>,
    pub swap_queue: Queue<(f64, f64)>,
}

impl App {
    pub fn new(name: &str, max_capacity_queue: usize) -> App {
        App {
            name: String::from(name),
            temps: vec![vec![]],
            disk_usage: vec![vec![]],
            should_quit: false,
            cpu_usage_queue: vec![Queue::with_capacity(max_capacity_queue); max_capacity_queue],
            cpu_usage_points: vec![vec![]; max_capacity_queue],
            max_capacity_queue,
            memory: Memory {
                memory_queue: Queue::with_capacity(max_capacity_queue),
                swap_queue: Queue::with_capacity(max_capacity_queue),
                free_memory: 0,
                free_swap: 0,
            },
        }
    }

    pub fn refresh<T: SystemExt>(&mut self, system: &mut T, logger: &mut Logger) {
        system.refresh_all();

        //Setting the Temperatures section data
        let temps: Vec<Vec<String>> = system
            .get_components()
            .iter()
            .map(|x| {
                let s: String = format!("{:?}", x);
                let (mut s1, mut s2): (String, String) = (String::from("k"), String::from(""));
                let mut flag = 0;

                for i in s.chars() {
                    if i == ':' {
                        flag = 1;
                        continue;
                    }

                    if flag == 0 {
                        s1.push(i);
                    } else {
                        s2.push(i);
                    }
                }
                vec![s1, s2]
            })
            .collect();
        self.temps = temps;

        //Setting the Disk Usage section data
        let disk_usage: Vec<Vec<String>> = system
            .get_disks()
            .iter()
            .map(|x| {
                let name = if let Some(nam) = x.get_name().to_str() {
                    String::from(nam)
                } else {
                    String::from("")
                };

                let mount_point = if let Some(mount_pt) = x.get_mount_point().to_str() {
                    String::from(mount_pt)
                } else {
                    String::from("")
                };

                let mut avb_space;
                let mut mb_flag = 0;
                let mut space = x.get_available_space() / 1000000;
                avb_space = space.to_string();
                if space > 1000 {
                    space = space / 1000;
                    avb_space = space.to_string();
                    mb_flag = 1;
                }

                if mb_flag == 0 {
                    avb_space += "MB";
                } else {
                    avb_space += "GB";
                }

                let mut v: Vec<String> = vec![];
                v.push(name);
                v.push(mount_point);
                v.push(avb_space);

                v
            })
            .collect();
        self.disk_usage = disk_usage;

        //Setting the cpu_usage section data
        for (i, cpu_no) in system.get_processors().iter().enumerate() {
            self.calculate_new_queue_processor(cpu_no, logger, i);
        }

        //Setting disk usage section data
        let memory_scaled: f64 = system.get_free_memory() as f64 / system.get_total_memory() as f64;
        App::calculate_new_queue_disk(
            memory_scaled * 10.0,
            logger,
            &mut self.memory.memory_queue,
            self.max_capacity_queue,
        );

        let swap_scaled: f64 = system.get_free_swap() as f64 / system.get_total_swap() as f64;
        App::calculate_new_queue_disk(
            swap_scaled,
            logger,
            &mut self.memory.swap_queue,
            self.max_capacity_queue,
        );

        if let Ok(_) = logger.add_log("\nIteration Over\n") {}
    }

    pub fn on_key(&mut self, key: &Key) {
        match key {
            Key::Char('q') => {
                self.should_quit = true;
            }
            Key::Char('Q') => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn calculate_new_queue_disk(
        val: f64,
        logger: &mut Logger,
        queue: &mut Queue<(f64, f64)>,
        max_capacity_queue: usize,
    ) {
        let mut log: String = String::from("");
        let mut q: Queue<(f64, f64)> = Queue::with_capacity(max_capacity_queue);

        if queue.len() < max_capacity_queue {
            let l = queue.len();

            match queue.peek() {
                Some(_) => {
                    if let Ok(_) = queue.queue((l as f64, val as f64)) {}
                    //log += &format!("Added: ({}, {}),\t", ele.0, current_usage);
                }
                None => {
                    log += &format!("Error adding: {}\t", l);
                }
            }

            if l == 0 {
                if let Ok(_) = queue.queue((0.0, 0.0)) {
                    //log += &format!("Added: (0, 0),\t");
                }
            }
        } else {
            let l = queue.len();
            if let Some(_) = queue.peek() {
                queue.force_queue((0.0, val as f64));
                //log += &format!("Added1: ({}, {}),\t", ele.0, current_usage);
            } else {
                log += &format!("Error adding: {}\t", l);
            }

            let v = queue.vec();

            for (i, ele) in v.iter().enumerate() {
                if let Ok(_) = q.queue((i as f64, ele.1)) {}

                log += &format!("({}, {}), ", i as f64, ele.1);
            }
            *queue = q;
        }

        //log += &format!("Usage Points Vector: {:?}\n", self.cpu_usage_points);

        if let Ok(_) = logger.add_log(log) {}
    }

    fn calculate_new_queue_processor(
        &mut self,
        cpu_no: &Processor,
        _logger: &mut Logger,
        i: usize,
    ) {
        let mut log: String = String::from("");
        let mut q: Queue<(f64, f64)> = Queue::with_capacity(self.max_capacity_queue);

        let current_usage: f64 = cpu_no.get_cpu_usage() as f64;

        if self.cpu_usage_points[i].len() < self.max_capacity_queue {
            let l = self.cpu_usage_points[i].len();

            match self.cpu_usage_queue[i].peek() {
                Some(_) => {
                    //let usage = current_usage - ele.1
                    if let Ok(_) =
                        self.cpu_usage_queue[i].queue((l as f64, (current_usage - 50.0) / 10.0))
                    {
                    }
                    //log += &format!("Added: ({}, {}),\t", ele.0, current_usage);
                }
                None => {
                    log += &format!("Error adding: {}\t", l);
                }
            }

            if l == 0 {
                if let Ok(_) = self.cpu_usage_queue[i].queue((0.0, 0.0)) {
                    //log += &format!("Added: (0, 0),\t");
                }
            }
        } else {
            let l = self.cpu_usage_points[i].len();
            if let Some(_) = self.cpu_usage_queue[i].peek() {
                //let usage = current_usage - ele.1;
                self.cpu_usage_queue[i].force_queue((0.0, (current_usage - 50.0) / 10.0));
            //log += &format!("Added1: ({}, {}),\t", ele.0, current_usage);
            } else {
                log += &format!("Error adding: {}\t", l);
            }

            let v = self.cpu_usage_queue[i].vec();

            for (i, ele) in v.iter().enumerate() {
                if let Ok(_) = q.queue((i as f64, ele.1)) {}

                log += &format!("({}, {}), ", i as f64, ele.1);
            }
            self.cpu_usage_queue[i] = q;
        }

        self.cpu_usage_points[i] = self.cpu_usage_queue[i].vec().clone();

        //log += &format!("Usage Points Vector: {:?}\n", self.cpu_usage_points);

        //if let Ok(_) = logger.add_log(log) {}
    }
}
