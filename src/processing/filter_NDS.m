% ------------------------------------------------------------------------%
% This function is used to filter the non-dominated solutions from the 
% current considered population
%
% Author:  Dr. Ke Li @ University of Birmingham
% Contact: keli.genius@gmail.com (http://www.cs.bham.ac.uk/~likw)
% Last modified: 01/10/2016
% ------------------------------------------------------------------------%

function filtered_pop = filter_NDS(cur_pop, whole_pop)

    num_objs = size(cur_pop, 2);
    index_array = zeros(size(cur_pop, 1), 1);
    for i = 1 : size(cur_pop, 1)
        for j = 1 : size(whole_pop, 1)
            flag = check_dominance(cur_pop(i, :), whole_pop(j, :), num_objs);
            if flag == -1
                index_array(i) = 1;
                break;
            end
        end
    end
    final_index = index_array == 0;
    filtered_pop = cur_pop(final_index, :);
end

% This function is used to check the dominance relationship between a and b
% 1: a dominates b | -1: b dominates a | 0: non-dominated
function dominance_flag = check_dominance(a, b, nobj)
    
    flag1 = 0;
    flag2 = 0;
    
    for i = 1 : nobj
        if (a(i) < b(i))
            flag1 = 1;
        else
            if (a(i) > b(i))
                flag2 = 1;
            end
        end
    end
    
    if (flag1 == 1 && flag2 == 0)
        dominance_flag = 1;
    elseif flag1 == 0 && flag2 == 1
        dominance_flag = -1;
    else
        dominance_flag = 0;
    end
    
end