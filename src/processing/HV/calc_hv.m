addpath('.');

close all
clear
clc
format long g

%% Parameters
num_objectives = 3;

runs = 30;

root_path = 'D:\Research\\NFV_MLS_Conf';
src_folder = [root_path, '\data\'];
results_folder = [root_path, '\results\'];

cd(src_folder);

% ---- Find nadir and utopian points ----
nadir = zeros(1, 3);
utopian = zeros(1, 3) + 10000000000;

file_search = dir('**\*.objs');

for j = 1:length(file_search)
    file = file_search(j);
    objs = get_objs(file.folder, file.name);
    
    if isempty(objs)
        continue
    end
    
    % Sometimes you get a file with only 1 feasible solution
    % The extra arguments to max/min handle that case
    nadir = max(nadir, max(objs, [], 1));
    utopian = min(utopian, min(objs, [], 1));
end

disp(nadir);
disp(utopian);

% --- Calculate HV ---
% Get folders that contain output files
sub_folders = split(genpath(src_folder), ';');

obj_folders = [];
for j = 1 : length(sub_folders)
    folder = sub_folders(j);
    
    item_path = fullfile(folder, '*.objs');
    item_path = item_path{1};
    items = dir(item_path);
    
    if ~isempty(items)
        obj_folders = [obj_folders, string(folder)];
    end
end

ref = zeros(1, num_objectives) + 1.000001;

% Calculate HV and write to file
for folder = obj_folders
    
    % Find objective files
    file_search = fullfile(folder, '*.objs');
    file_search = file_search{1};
    
    objectives_files = dir(file_search);
    objectives_files = natsortfiles({objectives_files.name});
    
    % Prepare output file
    out_file = fullfile(folder, 'HV.out');
    hv_out = fopen(out_file, 'w');
    
    for i = 1:length(objectives_files)
        name = objectives_files(i);
        name = name{1};
        objs = get_objs(folder, name);
        
        if ~isempty(objs)
            objs = filter_NDS(objs, objs);
            objs = unique(objs, 'rows');
            
            objs = (objs - utopian) ./ (nadir - utopian);
            
            hv = Hypervolume_MEX(objs, ref);
        else
            hv = 0;
        end
        
        num_evals = erase(name, '.objs');
        num_evals_idx = strfind(num_evals, '_');
        num_evals = extractBetween(num_evals, num_evals_idx + 1, length(num_evals));
        num_evals = str2double(num_evals);
        
        fprintf(hv_out,'%f,%f\n', num_evals, hv);
    end
    
    fclose(hv_out);
end

function objectives = get_objs(folder, file_name)
num_services_idx = strfind(file_name, '_');
num_services = extractBetween(file_name, 1, num_services_idx - 1);
num_services = str2double(num_services{1});

full_path = fullfile(folder, file_name);

try
    objectives = csvread(full_path);
    objectives = objectives(:, 1:3);
    objectives(:, 3) = objectives(:, 3) / num_services;
catch
    objectives = [];
end

end
