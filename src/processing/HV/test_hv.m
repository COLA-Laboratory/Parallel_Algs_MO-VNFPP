% root_path = '/media/joebillingsley/Data/projects/NFV_AG_Conf/data/Optimisation/';
% 
% file_a = fullfile(root_path, 'DCell', '420', '5', 'FLS', 'MOEAD', '10200_obj.out');
% file_b = fullfile(root_path, 'DCell', '8190', '5', 'VLS', 'IBEA', '10104_obj.out');
% 
% num_objectives = 3;
% 
% objs_a = csvread(file_a);
% objs_b = csvread(file_b);
% 
% objs_a = objs_a(:, 1:num_objectives);
% objs_b = objs_b(:, 1:num_objectives);
% 
% for i = 1 : num_objectives
%     utopian(i) = min(min(objs_a(:, i)), min(objs_b(:, i)));
%     nadir(i) = max(max(objs_a(:, i)), max(objs_b(:, i)));
% end
% 
% objs_a = (objs_a - utopian) ./ (nadir - utopian);
% objs_b = (objs_b - utopian) ./ (nadir - utopian);
% 
% ref = zeros(1, num_objectives) + 1.000001;


objs_a = [1, 2, 3];
objs_b = [4, 5, 6];

ref = zeros(1, 3) + 1.00000001;

hv_a = Hypervolume_MEX(objs_a, ref);
hv_b = Hypervolume_MEX(objs_b, ref);

disp(hv_a);
disp(hv_b);