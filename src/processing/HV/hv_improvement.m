addpath('.');

close all
clear
clc
format long g

%% Parameters
num_objectives = 3;

runs = 30;

root_path = 'D:\Research\NFV_AG_Journal';
src_folder = [root_path, '\data\potential_improvement\'];
results_folder = [root_path, '\results\'];

cd(src_folder);
objective_files = dir('**/*.csv');

% Find nadir and utopian points
nadir = zeros(1, 3);
utopian = zeros(1, 3) + 10000000000;

for i = 1:length(objective_files)
    file = objective_files(i);
    file_path = fullfile(file.folder, file.name);
    objs = get_objs(file_path);
    
    % Sometimes you get a file with only 1 feasible solution
    % The extra arguments to max/min handle that case
    nadir = max(nadir, max(objs, [], 1));
    utopian = min(utopian, min(objs, [], 1));
end

ref = zeros(1, num_objectives) + 1.000001;

% Calculate HV and other metrics
accs = ["0_005", "0_05", "0_5", "5", "50", "ut"];

hvs = zeros(23, 6);
uniques = zeros(23, 6);
nds = zeros(23, 6);

for i = 1:23
    for j = 1:length(accs)
        acc = accs(j);
        file_path = fullfile(src_folder, num2str(i-1), append(acc, '.csv'));
        src_objs = get_objs(file_path);
        
        uq_objs = unique(src_objs , 'rows');
        nd_objs = filter_NDS(uq_objs, uq_objs);
        
        nd_objs = (nd_objs - utopian) ./ (nadir - utopian);
        
        hv = Hypervolume_MEX(nd_objs, ref);
        
        hvs(i, j) = hv;
        uniques(i, j) = length(uq_objs);
        nds(i, j) = length(nd_objs);
    end
end

out_file = fullfile(results_folder, 'potential_improvement.csv');
writematrix(mean(hvs), out_file);

out_file = fullfile(results_folder, 'potential_improvement_uq.csv');
writematrix(mean(uniques), out_file);

out_file = fullfile(results_folder, 'potential_improvement_nds.csv');
writematrix(mean(nds), out_file);

% Plot HV
figure('Name','Normalised HV','NumberTitle','off');

accs_cat = categorical({'0.005', '0.05', '0.5', '5', '50', 'ut'});
Y = mean(hvs);
bar(accs_cat, Y);

xlabel('Accuracy'); 
ylabel('Normalised HV'); 

text(1:length(Y),Y,num2str(Y'),'vert','bottom','horiz','center'); 
box off

% Mean improvement
figure('Name','Mean Improvement','NumberTitle','off');

perc_diff = (hvs - hvs(:, 1)) / mean(hvs(:, 1)); % Positive numbers denote the setting improves on the base

accs_cat = categorical({'0.005', '0.05', '0.5', '5', '50', 'ut'});
Y = mean(perc_diff);
bar(accs_cat, Y);

xlabel('Accuracy'); 
ylabel('Percent Improvement'); 

text(1:length(Y),Y,num2str(Y'),'vert','bottom','horiz','center'); 
box off

function objectives = get_objs(file_path)

objectives = csvread(file_path);
objectives = objectives(1:120, 1:3);

end
