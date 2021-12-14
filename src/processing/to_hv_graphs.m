addpath('.');

close all
clear
clc
format long g
%% Parameters
num_objectives = 3;

runs = 30;

root_path = 'D:\Research\NFV_MLS_Conf\processed';

src_folder = fullfile(root_path, 'aggregate');
out_folder = fullfile(root_path, 'hv_graphs');

topologies = ["DCell", "FatTree", "LeafSpine"];
sizes = ["500", "1000", "2000", "4000", "8000"];
pop_sizes = ["32", "48", "80", "160", "320"];
algorithms = ["CNSGAII", "NSGAII", "PNSGAII", "PPLS"];

for topo = topologies
    dest_folder = fullfile(out_folder, topo);
    
    if ~exist(dest_folder, 'dir')
        mkdir(dest_folder);
    end
    
    for algo = algorithms
        dest_file = fullfile(dest_folder, algo);
        fileID = fopen(dest_file,'w');
        
        fprintf(fileID, 'PS HV\n');
        
        for size = sizes
            for pop_size = pop_sizes
                path = fullfile(src_folder, topo, size, pop_size, algo, 'aggregate.csv');
                data = readmatrix(path);
                mean = data(1);
                
                fprintf(fileID, '%s %s\n', num2str(pop_size), num2str(mean));
            end
            
            fprintf(fileID, '{} {}\n\n');
        end
    end
end