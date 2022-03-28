use crate::pipe::Step;

use super::PreParse;

pub struct Sort {}

impl PreParse for Sort {
    fn parse(list: Vec<Step>) -> Vec<Step> {
        let mut sort_list = list.clone();
        let mut step_first = Vec::new();
        let mut step_last = Vec::new();
        let mut by_reference_before = Vec::new();
        let mut by_reference_after = Vec::new();

        for item in list.iter() {
            if let Some(value) = item.tags.get("step") {
                match value.as_array() {
                    Some(value) => {
                        let val = value.get(0).unwrap();
                        let to = val.as_i64().unwrap() as usize;
                        if to != item.id {
                            sort_list.insert(to, item.clone());
                            if item.id < to {
                                sort_list.remove(item.id);
                            } else {
                                sort_list.remove(item.id + 1);
                            }
                        }
                    }
                    None => panic!("Unable to order modules by step {}.", item.id),
                }
            } else if item.tags.get("first").is_some() {
                step_first.push(item);
            } else if item.tags.get("last").is_some() {
                step_last.push(item);
            } else if let Some(value) = item.tags.get("before") {
                let refer = match value.as_array() {
                    Some(refer) => Some(refer.get(0).unwrap().as_str().unwrap().to_string()),
                    None => continue,
                };

                by_reference_before.push((refer, item.clone()));
            } else if let Some(value) = item.tags.get("after") {
                let refer = match value.as_array() {
                    Some(refer) => Some(refer.get(0).unwrap().as_str().unwrap().to_string()),
                    None => continue,
                };

                by_reference_after.push((refer, item.clone()));
            }
        }

        // First
        for item in step_first {
            let index = sort_list
                .iter()
                .enumerate()
                .find_map(|(i, s)| if s.id.eq(&item.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            sort_list.remove(index);
            sort_list.insert(0, item.clone());
        }

        // After
        for (refer, item) in by_reference_after {
            let to = match sort_list.iter().enumerate().find_map(|(i, a)| {
                if a.reference.eq(&refer) {
                    Some(i)
                } else {
                    None
                }
            }) {
                Some(index) => index + 1,
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };

            sort_list.insert(to, item.clone());

            match sort_list.iter().enumerate().find_map(|(i, a)| {
                if a.id.eq(&item.id) {
                    Some(i)
                } else {
                    None
                }
            }) {
                Some(index) => {
                    sort_list.remove(index);
                }
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };
        }

        let last = list.len() - 1;
        // Last
        for item in step_last {
            let index = sort_list
                .iter()
                .enumerate()
                .find_map(|(i, s)| if s.id.eq(&item.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            sort_list.remove(index);
            sort_list.insert(last, item.clone());
        }

        for (refer, item) in by_reference_before {
            let to = match sort_list.iter().enumerate().find_map(|(i, a)| {
                if a.reference.eq(&refer) {
                    Some(i)
                } else {
                    None
                }
            }) {
                Some(index) => index,
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };

            sort_list.insert(to, item.clone());

            match sort_list.iter().enumerate().find_map(|(i, a)| {
                if a.id.eq(&item.id) {
                    Some(i)
                } else {
                    None
                }
            }) {
                Some(index) => {
                    sort_list.remove(index);
                }
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };
        }

        sort_list
    }
}
