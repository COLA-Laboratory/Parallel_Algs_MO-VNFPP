addpath('.');

close all
clear
clc
format long g

%% Parameters
num_objectives = 3;

runs = 30;

% root_path = '/media/joebillingsley/Data/projects/NFV_PlacementModel_Journal';
root_path = 'D:\Research\NFV_MLS_Conf';

src_folder = fullfile(root_path, 'data');
out_folder = fullfile(root_path, 'processed', 'aggregate');

% Get list of all subfolders with obj files
all_folders = split(genpath(src_folder), ';');
obj_folders = [];

for i = 1 : length(all_folders) - 1
    folder = all_folders{i};
    file_search = fullfile(folder, 'running_time.out');
    items = dir(file_search);
    
    if ~isempty(items)
        seps = strfind(folder, filesep);
        folder = folder(1: seps(end)-1);
        
        if ~ismember(folder, obj_folders)
            obj_folders = [obj_folders, string(folder)];
        end
    end
end

for folder = obj_folders
    output = [];
    
    file_search = fullfile(folder, '*', 'running_time.out');
    time_files = dir(file_search);
    
    agg_time = [];
    
    for i = 1 : length(time_files)
        time_file = time_files(i);
        file = fullfile(time_file.folder, time_file.name);
        times = csvread(file);
        
        agg_time = [agg_time, times];
    end
    
    output(:, 1) = mean(agg_time, 2);
    output(:, 2) = std(agg_time, 0, 2);
    output(:, 3) = min(agg_time, [], 2);
    output(:, 4) = prctile(agg_time, 25, 2);
    output(:, 5) = median(agg_time, 2);
    output(:, 6) = prctile(agg_time, 75, 2);
    output(:, 7) = max(agg_time, [], 2);
    
    output = [["mean","stdev","min","lq","median","uq","max"]; output];
    
    dest_folder = fullfile(out_folder, erase(folder, src_folder));
    if ~exist(dest_folder, 'dir')
        mkdir(dest_folder);
    end
    
    out_file = fullfile(dest_folder, 'aggregate_times.csv');
    
    writematrix(output, out_file);
end