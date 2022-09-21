use lab_runtime::{
    declare_extension,
    extensions::{Extension, ExtensionType},
    lab::step::Step,
};

#[derive(Debug, Default, Clone)]
pub struct Custom {}

impl Custom {
    fn first(steps: Vec<Step>, list: &mut Vec<Step>) {
        for item in steps {
            let index = list
                .iter()
                .enumerate()
                .find_map(|(i, s)| if s.id.eq(&item.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            list.remove(index);
            list.insert(0, item.clone());
        }
    }

    fn last(steps: Vec<Step>, list: &mut Vec<Step>) {
        let last = list.len() - 1;
        // Last
        for item in steps {
            let index = list
                .iter()
                .enumerate()
                .find_map(|(i, s)| if s.id.eq(&item.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            list.remove(index);
            list.insert(last, item.clone());
        }
    }

    fn after(steps: Vec<(Option<String>, Step)>, list: &mut Vec<Step>) {
        for (refer, item) in steps {
            match list.iter().enumerate().find_map(
                |(i, a)| {
                    if a.id.eq(&item.id) {
                        Some(i)
                    } else {
                        None
                    }
                },
            ) {
                Some(index) => {
                    list.remove(index);
                }
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };

            let to = match list.iter().enumerate().find_map(|(i, a)| {
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

            list.insert(to, item.clone());
        }
    }

    fn before(steps: Vec<(Option<String>, Step)>, list: &mut Vec<Step>) {
        for (refer, item) in steps {
            match list.iter().enumerate().find_map(
                |(i, a)| {
                    if a.id.eq(&item.id) {
                        Some(i)
                    } else {
                        None
                    }
                },
            ) {
                Some(index) => {
                    list.remove(index);
                }
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };

            let to = match list.iter().enumerate().find_map(|(i, a)| {
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

            list.insert(to, item.clone());
        }
    }

    fn distribute(
        list: &mut Vec<Step>,
        sort_list: &mut Vec<Step>,
    ) -> (
        Vec<Step>,
        Vec<Step>,
        Vec<(Option<String>, Step)>,
        Vec<(Option<String>, Step)>,
    ) {
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
                step_first.push(item.clone());
            } else if item.tags.get("last").is_some() {
                step_last.push(item.clone());
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

        (
            step_first,
            step_last,
            by_reference_after,
            by_reference_before,
        )
    }
}

impl Extension for Custom {
    fn handler(&self, list: &mut Vec<Step>) {
        let mut sort_list = list.clone();
        let (step_first, step_last, after, before) = Self::distribute(list, &mut sort_list);

        Self::first(step_first, &mut sort_list);
        Self::last(step_last, &mut sort_list);
        Self::after(after, &mut sort_list);
        Self::before(before, &mut sort_list);

        list.clear();
        list.extend(sort_list);
    }

    fn extension_type(&self) -> ExtensionType {
        ExtensionType::PosParse
    }
}

declare_extension!(Custom, Custom::default);
