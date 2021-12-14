addpath(".");

close all
clear
clc
format long g

root_path = 'D:\Research\NFV_MLS_Conf';
src_folder = fullfile(root_path, 'data');
out_folder = fullfile(root_path, 'processed', 'pcp');

topologies = ["DCell", "FatTree", "LeafSpine"];
sizes = ["500", "1000", "2000", "4000", "8000"];
pop_sizes = ["32", "48", "80", "160", "320"];
algorithms = ["CNSGAII", "NSGAII", "PNSGAII", "PPLS"];
run = "10";

for topo = topologies
    dest_folder = fullfile(out_folder, topo);
    
    if ~exist(dest_folder, 'dir')
        mkdir(dest_folder);
    end
    
    for algo = algorithms
        dest_file = fullfile(dest_folder, algo);
        for size = sizes
            for pop_size = pop_sizes
                alg_folder = fullfile(src_folder, topo, size, pop_size, algo, run);
                file = dir(fullfile(alg_folder, '*.objs'));
                
                % Read in file and get non-dominated solutions
                objs = get_objs(file);
                
                % Write out data
                a_out = fullfile(out_folder, topo, size, pop_size, algo);
                make_if_not_exists(a_out);
                
                writematrix([["la", "pl", "en"]; objs], fullfile(a_out, 'pcp.csv'), 'Delimiter', ' ');
                
                sgtitle(file.name);
                %     parallelplot([ca_objs(:, 2), ca_objs(:, 1), ca_objs(:, 3)]);
                plot3(objs(:, 2), objs(:, 1), objs(:, 3), 'o');
            end
        end
    end
end

function objectives = get_objs(file)

file_path = fullfile(file.folder, file.name);

% Manually read CSV to handle 'Infeasible' values
fid = fopen(file_path);
lines = {};
tline = fgetl(fid);

objectives = [];
row = 1;

while ischar(tline)
    if contains(tline, 'Infeasible')
        tline = fgetl(fid);
        continue
    end
    
    s = split(tline, ',');
    
    objectives(row,1) = str2num(s{1});
    objectives(row,2) = str2num(s{2});
    objectives(row,3) = str2num(s{3});
    
    tline = fgetl(fid);
    
    row = row + 1;
end
fclose(fid);

objectives = filter_NDS(objectives, objectives);
objectives = unique(objectives, 'rows');

end

function make_if_not_exists(dest_folder)
if ~exist(dest_folder, 'dir')
    mkdir(dest_folder);
end
end
