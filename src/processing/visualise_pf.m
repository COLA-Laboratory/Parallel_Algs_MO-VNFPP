root_path = 'D:\Research\NFV_AG_Journal';
src_folder = [root_path, '\data\potential_improvement\'];

objective_files = dir([src_folder '**\*.csv']);

for i = 1:length(objective_files)
    file = objective_files(i);
    file_path = fullfile(file.folder, file.name);
    objs = get_objs(file_path);  
    
%     objs = filter_NDS(objs, objs);
    objs = unique(objs, 'rows');
    
    ttl = strrep(file.name, '_', '.');
    ttl = ttl(1:end-4);
    
    num_objs = num2str(length(objs));
    
    ttl = [ttl ' (' num_objs ')'];
    
    sgtitle(ttl);
    
    subplot(1,3,1);
    scatter(objs(:, 1), objs(:, 2));
    xlabel('Latency') 
    ylabel('Packet loss') 
    xlim([4 5])
    ylim([0 1])
    
    subplot(1,3,2);
    scatter(objs(:, 2), objs(:, 3));
    xlabel('Packet loss') 
    ylabel('Energy') 
    xlim([0 1])
    ylim([10000 30000])
    
    subplot(1,3,3);
    scatter(objs(:, 1), objs(:, 3));
    xlabel('Latency') 
    ylabel('Energy')
    xlim([4 5])
    ylim([10000 30000])
    
    pause(2)
end

function objectives = get_objs(file_path)

objectives = csvread(file_path);
objectives = objectives(1:120, 1:3);

end