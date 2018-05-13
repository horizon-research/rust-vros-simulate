use ds::{Frame, Viewport};
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::error::Error;
use std::path::Path;
use std::f64;

extern crate serde;
extern crate serde_json;

#[derive(Debug, Copy, Clone)]
enum CacheLevel {
    LevelOne,
    LevelTwo,
    LevelThree,
}

#[derive(Debug, Copy, Clone)]
enum FrameSize {
    Small,
    Full,
}

#[derive(Debug, Copy, Clone)]
struct Hit {
    index: usize,
    ratio: f64,
    path: usize,
    cache_level: CacheLevel,
    width: usize,
    height: usize,
}

#[derive(Clone, Deserialize, Debug)]
pub struct PowerConstants {
    name: String,
    value: f64,
}

pub struct Simulator {
    user_file: String,
    dump_file: String,
    cluster_json: String,
    threshold: f64,
    segment: usize,
    fov_width: usize,
    fov_height: usize,
    path_list: Vec<Vec<Viewport>>,
    user_fov_list: Vec<Viewport>,
    level_two_width: usize,
    level_two_height: usize,
    hit_list: Vec<Hit>,
    level_one_power_constant: Vec<PowerConstants>,
    full_size_power_constant: Vec<PowerConstants>,
    opt_flag: bool,
    ignored_level_two: usize,
    wifi_pc: f64,
    soc_pc: f64,
}

#[derive(Deserialize, Debug)]
struct VideoObject {
    frame_start: usize,
    frame_end: usize,
    size: usize,
    cluster: Vec<usize>,
}

fn read_json_cluster_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<VideoObject>, Box<Error>> {
    let file = File::open(path)?;

    // Read the JSON contents of the file as an instance of `Vec[VideoObject]`.
    let u = serde_json::from_reader(file)?;

    // Return the `VideoObject`.
    Ok(u)
}

impl Simulator {
    pub fn new(user_file: &String, dump_file: &String, cluster_json: &String, threshold: f64,
               segment: usize, fov_width: usize, fov_height: usize, level_two_width: usize,
               level_two_height: usize, full_size_power_constant: Vec<PowerConstants>,
               level_one_power_constant: Vec<PowerConstants>, opt_flag: bool) -> Self {
        let mut sim = Simulator {
            user_file: user_file.to_string(),
            dump_file: dump_file.to_string(),
            cluster_json: cluster_json.to_string(),
            threshold,
            segment,
            fov_width,
            fov_height,
            path_list: vec![],
            user_fov_list: vec![],
            level_two_width,
            level_two_height,
            hit_list: vec![],
            level_one_power_constant,
            full_size_power_constant,
            opt_flag,
            ignored_level_two: 0,
            wifi_pc: 0.0,
            soc_pc: 0.0,
        };
        sim.parse_tracing_to_path_list();
        sim.parse_user_data();
        sim
    }

    // update self.path_list
    fn parse_tracing_to_path_list(&mut self) {
        let file = File::open(&self.dump_file).unwrap();
        let buf_reader = BufReader::new(&file);
        let mut traces: Vec<Viewport> = vec![];
        let mut frame_id = 0;
        let mut frame_list: Vec<Frame> = vec![];

        for line in buf_reader.lines() {
            let line = line.unwrap();
            let id_vec: Vec<&str> = line.split(" ").collect();
            frame_id = (&id_vec[0]).parse::<i32>().unwrap();
            let object_id = (&id_vec[1]).parse::<i32>().unwrap();

            let coord: Vec<&str> = id_vec[2].split(",").collect();
            let x = (&coord[0]).parse::<i32>().unwrap();
            let y = (&coord[1]).parse::<i32>().unwrap();
            let width = (&coord[2]).parse::<usize>().unwrap();
            let height = (&coord[3]).parse::<usize>().unwrap();
            let viewport = Viewport::new(100, x, y, width, height);

            if object_id == 0 {
                if frame_id != 1 {
                    frame_list.push(Frame::new(frame_id, &traces));
                }
                traces.clear();
            }
            traces.push(viewport);
        }
        // viewport in frame_list is not normalized using our fov size yet
        frame_list.push(Frame::new(frame_id, &traces));

        // integrate cluster_json and trace dump
        let video_objects = read_json_cluster_from_file(&self.cluster_json).unwrap();
        for video_object in video_objects {
            // frame_start and frame_end - 1 is for normalize the start id in trace dump file to 0
            // so that we have the same start id as we get from user_view_port_result
            let start = video_object.frame_start - 1;
            let end = video_object.frame_end - 1;
            let mut _frame_id = start;
            let mut path: Vec<Viewport> = vec![];

            // iterate all the frames from dumping data
            for frame in frame_list[start..end].iter() {
                for cluster in &video_object.cluster {
                    if *cluster < frame.traces.len() {
                        let v = frame.traces[*cluster];
                        path.push(Viewport::create_new_with_size(&v, self.fov_width, self.fov_height));
                    }
                }
                self.path_list.push(path.clone());
                path.clear();
            }
//            println!("{}: {:?}", start, self.path_list[start]);
        }
    }

    // update user_fov_list
    fn parse_user_data(&mut self) {
        let file = File::open(&self.user_file).unwrap();
        let buf_reader = BufReader::new(file);

        for line in buf_reader.lines() {
            let line = line.unwrap();
            let line_split: Vec<&str> = line.split(" ").collect();
//            let key = (&line_split[0]).parse::<usize>().unwrap();
            let conf = (&line_split[1]).parse::<i32>().unwrap();

            let extract: Vec<&str> = line_split[2].split(",").collect();
            let x = (&extract[0]).parse::<i32>().unwrap();
            let y = (&extract[1]).parse::<i32>().unwrap();
            let width = (&extract[2]).parse::<usize>().unwrap();
            let height = (&extract[3]).parse::<usize>().unwrap();
            let u_fov = Viewport::new(conf, x, y, width, height);
//            // assume user_viewport file has key start from 0 and add one consecutively
            self.user_fov_list.push(u_fov);
        }
//        println!("{:?}", self.user_fov_list);
    }

    fn compare_from_level_one(&self, fov: &Viewport, user_fov: &Viewport, index: usize, path: usize, width: usize, height: usize, ignored_level_two: &mut usize) -> (Hit, CacheLevel) {
        let ratio = fov.get_cover_result(user_fov);
        let hit: Hit;
        if ratio >= self.threshold {
//            println!("L1 hit {} at {}", index, ratio);
            hit = Hit {
                index,
                ratio,
                cache_level: CacheLevel::LevelOne,
                path,
                width,
                height,
            };
            (hit, CacheLevel::LevelOne)
        } else {
            // predict if level-two is actually miss before get the data from cloud
            if self.opt_flag {
//                println!("L1 miss {} at {}", index, ratio);
                let level_two_viewport = Viewport::create_new_with_size(&fov, self.level_two_width, self.level_two_height);
                if level_two_viewport.get_cover_result(user_fov) < self.threshold {
                    *ignored_level_two += 1;
                    self.compare_from_level_three(index, path)
                } else {
                    self.compare_from_level_two(&fov, &user_fov, index, path)
                }
            } else {
//                println!("L1 miss {} at {}", index, ratio);
                if self.is_hierarchical() {
                    self.compare_from_level_two(&fov, &user_fov, index, path)
                } else {
                    self.compare_from_level_three(index, path)
                }
            }
        }
    }

    fn compare_from_level_two(&self, fov: &Viewport, user_fov: &Viewport, index: usize, path: usize) -> (Hit, CacheLevel) {
        let level_one_ratio = fov.get_cover_result(user_fov);
        let hit: Hit;
        let level_two_viewport = Viewport::create_new_with_size(&fov, self.level_two_width, self.level_two_height);
        let level_two_ratio = level_two_viewport.get_cover_result(user_fov);
        if level_two_ratio >= self.threshold {
//            println!("L2 hit {} at {}", index, level_two_ratio);
            hit = Hit {
                index,
                ratio: level_two_ratio,
                cache_level: CacheLevel::LevelTwo,
                path,
                width: self.level_two_width,
                height: self.level_two_height,
            };
            (hit, CacheLevel::LevelTwo)
        } else {
//            println!("L2 miss {} at {}", index, level_two_ratio);
            if level_two_ratio < level_one_ratio {
                println!("index: {}, l1 ratio: {}, l2 ratio: {}", index, level_one_ratio, level_two_ratio);
                println!("l1 {:?}", fov);
                println!("l2 {:?}", level_two_viewport);
                println!("user {:?}", user_fov);
                assert!(false);
            }
            self.compare_from_level_three(index, path)
        }
    }

    fn compare_from_level_three(&self, index: usize, path: usize) -> (Hit, CacheLevel) {
//        println!("L3 hit {} at {}", index, 1);
        let hit: Hit;
        hit = Hit {
            index,
            ratio: 1.0,
            cache_level: CacheLevel::LevelThree,
            path,
            width: 3840,
            height: 2160,
        };
        (hit, CacheLevel::LevelThree)
    }

    fn is_hierarchical(&self) -> bool {
        if self.fov_width == self.level_two_width && self.fov_height == self.level_two_height {
            false
        } else {
            true
        }
    }

    // simulate with hierarchical or non-hierarchical with segment and threshold implicitly
    pub fn simulate(&mut self) {
        let mut current_path: Option<usize> = None;
        let mut local_ignored_level_two;
        let mut hit_cache_pair: (Hit, CacheLevel) = (Hit {
            index: 0,
            ratio: 0.0,
            cache_level: CacheLevel::LevelOne,
            path: 0,
            width: 0,
            height: 0,
        }, CacheLevel::LevelOne);
        for (k, user_fov) in self.user_fov_list.iter().enumerate() {
            let mut max_ratio: f64 = f64::NEG_INFINITY;
            let mut max_ratio_path: Option<usize> = None;
            let mut temp_viewport: Option<&Viewport> = None;
            let width = self.fov_width;
            let height = self.fov_height;
            local_ignored_level_two = 0;

            if self.path_list.len() > k {
                for (path, path_viewport) in self.path_list[k].iter().enumerate() {
                    let current_ratio = path_viewport.get_cover_result(user_fov);
                    if max_ratio < current_ratio {
                        max_ratio = current_ratio;
                        max_ratio_path = Some(path);
                        temp_viewport = Some(path_viewport);
                    }
                }
                if k % self.segment == 0 {
                    // the first frame in the segment
                    current_path = max_ratio_path;
                    hit_cache_pair = self.compare_from_level_one(&temp_viewport.unwrap(), &user_fov, k, max_ratio_path.unwrap(), width, height, &mut local_ignored_level_two);
                } else {
                    // the rest frames except for the first one in the segment
//                    println!("k: {}, path: {}, ratio: {}", k, max_ratio_path.unwrap(), max_ratio);

                    if self.is_hierarchical() {
                        // hierarchical
                        match hit_cache_pair.1 {
                            CacheLevel::LevelOne => {
                                if current_path == max_ratio_path {
                                    hit_cache_pair = self.compare_from_level_one(&temp_viewport.unwrap(), &user_fov, k, max_ratio_path.unwrap(), width, height, &mut local_ignored_level_two);
                                } else {
                                    hit_cache_pair = self.compare_from_level_three(k, max_ratio_path.unwrap());
                                }
                            }
                            CacheLevel::LevelTwo => {
                                if current_path == max_ratio_path {
                                    hit_cache_pair = self.compare_from_level_two(&temp_viewport.unwrap(), &user_fov, k, max_ratio_path.unwrap());
                                } else {
                                    hit_cache_pair = self.compare_from_level_three(k, current_path.unwrap());
                                }
                            }
                            CacheLevel::LevelThree => {
                                hit_cache_pair = self.compare_from_level_three(k, current_path.unwrap());
                            }
                        }
                    } else {
                        match hit_cache_pair.1 {
                            CacheLevel::LevelOne => {
                                if current_path == max_ratio_path {
                                    hit_cache_pair = self.compare_from_level_one(&temp_viewport.unwrap(), &user_fov, k, max_ratio_path.unwrap(), width, height, &mut local_ignored_level_two);
                                } else {
                                    hit_cache_pair = self.compare_from_level_three(k, max_ratio_path.unwrap());
                                }
                            }
                            CacheLevel::LevelTwo => assert!(false),
                            CacheLevel::LevelThree => hit_cache_pair = self.compare_from_level_three(k, current_path.unwrap()),
                        }
                    }
                }
                self.hit_list.push(hit_cache_pair.0.clone());
                self.ignored_level_two += local_ignored_level_two;
            }
        }
//        assert_eq!(self.hit_list.len(), self.user_fov_list.len());
//        println!("The amount of level two that is ignored {}, total level-three {:?}", self.ignored_level_two, self.get_hit_counts()[2]);

        // fill wifi_pc and soc_pc
        self.power_consumption();
    }

    pub fn get_hit_counts(&self) -> Box<[usize; 3]> {
        let mut count_arr: Box<[usize; 3]> = Box::new([0, 0, 0]);
        (&self.hit_list).iter().for_each(|&x| match x.cache_level {
            CacheLevel::LevelOne => count_arr[0] += 1,
            CacheLevel::LevelTwo => count_arr[1] += 1,
            CacheLevel::LevelThree => count_arr[2] += 1,
        });
//        println!("{:?}", count_arr);
        count_arr
    }

    pub fn get_hit_ratios(&self) -> Box<[f64; 3]> {
        let hit_counts = self.get_hit_counts().to_vec();
        let hit_len = self.hit_list.len();
        let mut hit_ratios: Box<[f64; 3]> = Box::new([0.0, 0.0, 0.0]);
        hit_ratios[0] = hit_counts[0] as f64 / hit_len as f64;
        hit_ratios[1] = hit_counts[1] as f64 / hit_len as f64;
        hit_ratios[2] = hit_counts[2] as f64 / hit_len as f64;
        hit_ratios
    }

    pub fn get_accumulate_hit_ratio(&self) -> Box<[f64; 3]> {
        let hit_len = self.hit_list.len();
        let hit_count_arr = self.get_hit_counts();
        let mut acc_hit_ratio: Box<[f64; 3]> = Box::new([0.0, 0.0, 0.0]);
        acc_hit_ratio[0] = hit_count_arr[0] as f64 / hit_len as f64;
        acc_hit_ratio[1] = acc_hit_ratio[0] + (hit_count_arr[1] as f64 / hit_len as f64);
        acc_hit_ratio[2] = 1.0;
//        println!("{:?}", acc_hit_ratio);
        acc_hit_ratio
    }

    fn get_wifi_power_constant(&self, video_name: &str, size: FrameSize) -> f64 {
        let mut wifi_name: String = video_name.to_owned().to_string();
        wifi_name.push_str("_WIFI");
        match size {
            FrameSize::Small => self.level_one_power_constant.iter().find(|&x| x.name == wifi_name).unwrap().value,
            FrameSize::Full => self.full_size_power_constant.iter().find(|&x| x.name == wifi_name).unwrap().value
        }
    }

    fn get_soc_power_constant(&self, video_name: &str, size: FrameSize) -> f64 {
        let mut soc_name: String = video_name.to_owned().to_string();
        soc_name.push_str("_SOC");
        match size {
            FrameSize::Small => self.level_one_power_constant.iter().find(|&x| x.name == soc_name).unwrap().value,
            FrameSize::Full => self.full_size_power_constant.iter().find(|&x| x.name == soc_name).unwrap().value
        }
    }

    // small is for level one power constant, big is for full size power constant
    fn level_two_interpolate(&self, small: f64, big: f64) -> f64 {
        let l1_resolution = self.fov_width * self.fov_height;
        let l2_resolution = self.level_two_width * self.level_two_height;
        let full_resolution = 3840 * 2160;
        let x = l2_resolution - l1_resolution;
        let y = full_resolution - l2_resolution;
        ((x as f64 * small) / (x + y) as f64) + ((y as f64 * big) / (x + y) as f64)
    }

    pub fn power_consumption(&mut self) {
        // extract name from user_file
        // which for example could be: user_viewport_result/Elephant-training-2bpICIClAIg/uid-a413ecca-3822-47b3-92f3-2e2fbe8470c0.txt
        let video_name: &str = {
            let temp_name: &str = self.user_file.split("/").collect::<Vec<_>>()[1];
            temp_name.split("-").collect::<Vec<_>>()[0]
        };

        // get power constant value
        let small_wifi_constant = self.get_wifi_power_constant(&video_name, FrameSize::Small);
        let full_wifi_constant = self.get_wifi_power_constant(&video_name, FrameSize::Full);
        let small_soc_constant = self.get_soc_power_constant(&video_name, FrameSize::Small);
        let full_soc_constant = self.get_soc_power_constant(&video_name, FrameSize::Full);
//        println!("PPPPP {} {} {} {} {} {}",
//                 small_wifi_constant, self.level_two_interpolate(small_wifi_constant, full_wifi_constant), full_wifi_constant,
//                 small_soc_constant, self.level_two_interpolate(small_soc_constant, full_soc_constant), full_soc_constant);

        // Power constant for each level
        let cache_hit_ratios = self.get_hit_ratios();
        let wifi_level_one_power_constant = small_wifi_constant;
        let wifi_level_two_power_constant = self.level_two_interpolate(small_wifi_constant, full_wifi_constant);
        let wifi_level_three_power_constant = full_wifi_constant;
        let soc_level_one_power_constant = small_soc_constant;
        let soc_level_two_power_constant = self.level_two_interpolate(small_soc_constant, full_soc_constant);
        let soc_level_three_power_constant = full_soc_constant;

        // Computation for wifi:
        // Since we got hit rate on each level, the hit rate level of each frame means that they
        // need to transmit data cumulatively. For instance, frame 1 hit at level 2, therefore, in
        // our VR system, we need to transmit both frame of level 1 and level 2 size. You might think
        // that why don't we need to consider about the whole segment for computing power consumption.
        // That is because we've firstly computing the hit rate in the sense of having segment in
        // our VR system.
        //
        // Computation for optimized wifi:
        // In this version, we could prevent the system from fetching level-2 cache by using the
        // metadata from client sensor.
        if self.is_hierarchical() && (!self.opt_flag) {
            self.wifi_pc = {
                let first_level = cache_hit_ratios[0] * wifi_level_one_power_constant;
                let second_level = cache_hit_ratios[1] * (wifi_level_one_power_constant + wifi_level_two_power_constant);
                let third_level = cache_hit_ratios[2] * (wifi_level_one_power_constant + wifi_level_two_power_constant + wifi_level_three_power_constant);
                first_level + second_level + third_level
            };
            self.soc_pc = {
                let first_level = cache_hit_ratios[0] * soc_level_one_power_constant;
                let second_level = cache_hit_ratios[1] * soc_level_two_power_constant;
                let third_level = cache_hit_ratios[2] * soc_level_three_power_constant;
                first_level + second_level + third_level
            };
        } else if self.is_hierarchical() && self.opt_flag {
            self.wifi_pc = {
                let first_level = cache_hit_ratios[0] * wifi_level_one_power_constant;
                let third_level = cache_hit_ratios[2] * (wifi_level_one_power_constant + wifi_level_two_power_constant + wifi_level_three_power_constant);
                // save is the ratio that we (ignored before fetch / total) * power constant
                // let save = (self.ignored_level_two as f64 / self.hit_list.len() as f64) * wifi_level_two_power_constant;
                // println!("save {}", save);
                first_level + third_level
            };
            self.soc_pc = {
                let first_level = cache_hit_ratios[0] * soc_level_one_power_constant;
                let second_level = cache_hit_ratios[1] * soc_level_two_power_constant;
                let third_level = cache_hit_ratios[2] * soc_level_three_power_constant;
                first_level + second_level + third_level
            };
        } else {
            self.wifi_pc = {
                let first_level = cache_hit_ratios[0] * wifi_level_one_power_constant;
                let third_level = cache_hit_ratios[2] * (wifi_level_one_power_constant + wifi_level_three_power_constant);
                assert_eq!(cache_hit_ratios[1], 0.0);
                first_level + third_level
            };
            self.soc_pc = {
                let first_level = cache_hit_ratios[0] * soc_level_one_power_constant;
                let third_level = cache_hit_ratios[2] * soc_level_three_power_constant;
                assert_eq!(cache_hit_ratios[1], 0.0);
                first_level + third_level
            };
        }

//        self.print_power_consumption();
    }

    pub fn print_power_consumption(&self) {
        println!("{} {}", self.wifi_pc, self.soc_pc);
    }

    pub fn get_wifi_pc(&self) -> f64 {
        self.wifi_pc
    }

    pub fn get_soc_pc(&self) -> f64 {
        self.soc_pc
    }
}
