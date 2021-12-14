mod algorithms;
mod models;
mod operators;
mod utilities;

use algorithms::{cnsgaii, nsgaii, pnsgaii, pplsd};
use utilities::stopwatch::Stopwatch;

use models::{
    datacentre::{Datacentre, Topology},
    queueing_model::QueueingModel,
    routing,
    service::{Service, VNF},
};
use operators::{
    distance_matrix, evaluation::QueueingEval, initialisation::ServiceAwareInitialisation,
    mapping::ServiceToRouteMapping, mutation::AddRemoveSwapMutation,
    neighbour_gen::AddSwapNeighbour, placement_strategies::FirstFit, solution::Solution,
};
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use routing::RoutingTable;
use std::{
    fs::{self, File, OpenOptions},
    io::{prelude::*, BufReader, BufWriter},
    path::PathBuf,
};

use crate::operators::crossover::UniformCrossover;

fn main() {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("Config"))
        .unwrap()
        .merge(config::Environment::with_prefix("APP"))
        .unwrap();

    // Get output folder
    let results_folder: String = settings.get("results_folder").unwrap();
    let results_folder = PathBuf::new().join(&results_folder);

    let max_evaluations: usize = settings.get("max_evaluations").unwrap();

    if settings.get("test_num_cores").unwrap() {
        let mut file = get_file(&results_folder, "NumCores.txt").unwrap();
        writeln!(&mut file, "{}", num_cpus::get()).unwrap();

        return;
    }

    if settings.get("fat_tree").unwrap() {
        println!("Starting Fat Tree");
        run_basic_tests(Topology::FatTree, &results_folder, max_evaluations);
    }

    if settings.get("dcell").unwrap() {
        println!("Starting DCell");
        run_basic_tests(Topology::DCell, &results_folder, max_evaluations);
    }

    if settings.get("leaf_spine").unwrap() {
        println!("Starting Leaf Spine");
        run_basic_tests(Topology::LeafSpine, &results_folder, max_evaluations);
    }
}

fn run_basic_tests(topology: Topology, results_folder: &PathBuf, max_evaluations: usize) {
    let scales = [16000];

    let sw_sr_base = 20.0;
    let sw_ql_base = 20;
    let accuracy = 5.0;
    let converged_iterations = 10;
    let active_cost = 30.0;
    let idle_cost = 10.0;

    let utilisation = 0.6;

    for scale in scales.iter() {
        let (dc, rt) = load_topology(&topology, *scale);

        let sw_sr = sw_sr_base * dc.num_ports as f64;
        let sw_ql = sw_ql_base * dc.num_ports;

        // Mapping + Fitness function
        let num_nearest = dc.num_servers;
        let dm = distance_matrix::build_cache(&dc, num_nearest);

        let capacities = vec![100; dc.num_servers];

        let qm = QueueingModel::new(
            &dc,
            sw_sr,
            sw_ql,
            accuracy,
            converged_iterations,
            active_cost,
            idle_cost,
        );

        for pi in 0..30 {
            let problem_folder = results_folder
                .join(topology.to_string())
                .join(scale.to_string());

            let services = create_problem_instance(dc.num_servers, utilisation);

            let node_selection = FirstFit::new();

            // --- Genetic Operators ---
            let evaluate = QueueingEval::new(
                qm.clone(),
                &rt,
                &dm,
                &capacities,
                &services,
                node_selection.clone(),
            );

            // Initialisation
            let init_pop = ServiceAwareInitialisation::new(&services, dc.num_servers);

            // Mapping
            let strm = ServiceToRouteMapping::new(node_selection.clone(), &capacities, &dm, &rt);

            // Mutation
            let pm = 0.4;

            let items = services.iter().map(|s| s).collect();

            let mutation = AddRemoveSwapMutation::new(items, pm);

            // Crossover
            let pc = 0.4;
            let crossover = UniformCrossover::new(pc);

            // --- NSGA-II ---
            let alg_folder = problem_folder.join("NSGAII").join(pi.to_string());

            let mut ns_st = Stopwatch::new();
            ns_st.start();

            nsgaii::run(
                &init_pop,
                &strm,
                &evaluate,
                &mutation,
                &crossover,
                128,
                max_evaluations,
                |evaluations, pop| {
                    let time = ns_st.stop();

                    let file_name = format!("{}_{}.objs", services.len(), evaluations);
                    print_population_objectives(&alg_folder, file_name, pop).unwrap();

                    let mut file = get_file(&alg_folder, "running_time.out").unwrap();
                    write!(file, "{}", time).unwrap();
                },
            );

            // --- C-NSGA-II ---
            let alg_folder = problem_folder.join("CNSGAII").join(pi.to_string());

            let mut cs_st = Stopwatch::new();
            cs_st.start();

            cnsgaii::run(
                &init_pop,
                &strm,
                &evaluate,
                &mutation,
                &crossover,
                128,
                max_evaluations,
                |evaluations, pop| {
                    let time = cs_st.stop();

                    let file_name = format!("{}_{}.objs", services.len(), evaluations);
                    print_population_objectives(&alg_folder, file_name, pop).unwrap();

                    let mut file = get_file(&alg_folder, "running_time.out").unwrap();
                    write!(file, "{}", time).unwrap();
                },
            );

            // --- P-NSGA-II ---
            let alg_folder = problem_folder.join("PNSGAII").join(pi.to_string());
            let num_epochs = 10;

            let mut ps_st = Stopwatch::new();
            ps_st.start();

            pnsgaii::run(
                &init_pop,
                &strm,
                &evaluate,
                &mutation,
                &crossover,
                128,
                max_evaluations,
                num_epochs,
                |evaluations, pop| {
                    let time = ps_st.stop();

                    let file_name = format!("{}_{}.objs", services.len(), evaluations);
                    print_population_objectives(&alg_folder, file_name, pop).unwrap();

                    let mut file = get_file(&alg_folder, "running_time.out").unwrap();
                    write!(file, "{}", time).unwrap();
                },
            );

            // --- PPLS/D ---
            let items = services.iter().map(|s| s).collect();
            let neighbour_gen = AddSwapNeighbour::new(items);

            let alg_folder = problem_folder.join("PPLS").join(pi.to_string());

            let mut pp_st = Stopwatch::new();
            pp_st.start();

            pplsd::run(
                &init_pop,
                &strm,
                &evaluate,
                &neighbour_gen,
                16,
                max_evaluations,
                10,
                3,
                |evaluations, pop| {
                    let time = pp_st.stop();

                    let file_name = format!("{}_{}.objs", services.len(), evaluations);
                    print_population_objectives(&alg_folder, file_name, pop).unwrap();

                    let mut file = get_file(&alg_folder, "running_time.out").unwrap();
                    write!(file, "{}", time).unwrap();
                },
            );

            // // --- SPPLS ---
            // let items = services.iter().map(|s| s).collect();
            // let neighbour_gen = AddSwapNeighbour::new(items);

            // let alg_folder = problem_folder.join("SPPLS").join(pi.to_string());

            // let mut sp_st = Stopwatch::new();
            // sp_st.start();

            // sppls::run(
            //     &init_pop,
            //     &strm,
            //     &evaluate,
            //     &neighbour_gen,
            //     128,
            //     max_evaluations,
            //     10,
            //     3,
            //     |evaluations, pop| {
            //         let time = sp_st.stop();

            //         let file_name = format!("{}_{}.objs", services.len(), evaluations);
            //         print_population_objectives(&alg_folder, file_name, pop).unwrap();

            //         let mut file = get_file(&alg_folder, "running_time.out").unwrap();
            //         write!(file, "{}", time).unwrap();
            //     },
            // );
        }
    }
}

fn create_problem_instance(num_servers: usize, utilisation: f64) -> Vec<Service> {
    // Problem parameter settings
    let mean_service_len = 5.0;
    let variance_service_len = 1.0;

    let mean_prod_rate = 10.0;
    let variance_prod_rate = 3.0;

    let mean_service_rate = 10.0;
    let variance_service_rate = 3.0;

    let queue_length = 20;

    let mean_size = 40.0;
    let variance_size = 10.0;

    // Distributions
    let service_len_distr = Normal::new(mean_service_len, variance_service_len).unwrap();
    let prod_rate_distr = Normal::new(mean_prod_rate, variance_prod_rate).unwrap();
    let service_rate_distr = Normal::new(mean_service_rate, variance_service_rate).unwrap();
    let size_distr = Normal::new(mean_size, variance_size).unwrap();

    let num_services =
        (utilisation * (1.0 / mean_service_len) * num_servers as f64).max(1.0) as usize;

    let mut rng = thread_rng();

    loop {
        let mut vnf_id = 0;
        let mut services = Vec::new();
        for service_id in 0..num_services {
            let prod_rate: f64 = prod_rate_distr.sample(&mut rng);
            let prod_rate = prod_rate.max(2.0);

            let mut service = Service {
                id: service_id,
                prod_rate: prod_rate,
                vnfs: Vec::new(),
            };

            let num_vnfs = service_len_distr.sample(&mut rng).max(2.0).min(12.0);
            let num_vnfs = num_vnfs as usize;

            for _ in 0..num_vnfs {
                let service_rate: f64 = service_rate_distr.sample(&mut rng);
                let service_rate = service_rate.max(2.0);

                let size: f64 = size_distr.sample(&mut rng);
                let size = size.min(100.0).max(1.0) as usize;

                service.vnfs.push(VNF {
                    service_rate,
                    queue_length,
                    size,
                });

                vnf_id = vnf_id + 1;
            }

            services.push(service);
        }

        let mut used_capacity = 0;
        for service in &services {
            for vnf in &service.vnfs {
                used_capacity = used_capacity + vnf.size;
            }
        }

        // Filter out some unsolveable problems
        let total_capacity = 100 * num_servers;
        if used_capacity <= total_capacity {
            return services;
        }
    }
}

fn load_topology(topology: &Topology, size: usize) -> (Datacentre, Vec<RoutingTable>) {
    let file = File::open(format!("topology/{}_{}.dat", topology, size)).unwrap();
    let reader = BufReader::new(file);
    let dc: Datacentre = bincode::deserialize_from(reader).unwrap();

    let file = File::open(format!("topology/{}_routing_{}.dat", topology, size)).unwrap();
    let reader = BufReader::new(file);
    let rt: Vec<RoutingTable> = bincode::deserialize_from(reader).unwrap();

    (dc, rt)
}

fn print_population_objectives<X>(
    folder: &PathBuf,
    file_name: String,
    pop: &Vec<Solution<X>>,
) -> std::io::Result<()> {
    let mut file = get_file(&folder, &file_name).unwrap();

    for ind in pop {
        let objectives = &ind.objectives;

        if objectives.is_feasible() {
            let objectives = objectives.unwrap();

            for (i, objective) in objectives.iter().enumerate() {
                write!(file, "{}", objective)?;

                if i < objectives.len() - 1 {
                    write!(file, ",")?;
                }
            }
        } else {
            write!(file, "Infeasible")?;
        }

        writeln!(file)?;
    }

    Ok(())
}

fn get_file(folder: &PathBuf, file: &str) -> std::io::Result<BufWriter<File>> {
    println!("{:?}", folder);

    fs::create_dir_all(folder).unwrap();
    let path = folder.join(file);

    let file = OpenOptions::new().write(true).create(true).open(path)?;

    Ok(BufWriter::new(file))
}
