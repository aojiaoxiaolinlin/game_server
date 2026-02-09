use std::{collections::HashMap, sync::LazyLock};

use common::sprites::attributes::Attribute;

/// 正常属性之间的克制关系倍数
const NORMAL: f32 = 1.0;

static ATTRIBUTE_RELATIONSHIP: LazyLock<HashMap<Attribute, HashMap<Attribute, f32>>> =
    LazyLock::new(|| {
        // 读取配置文件
        let config = include_str!("../configs/attribute_relationship.json");
        // 解析配置文件
        let config: HashMap<Attribute, HashMap<Attribute, f32>> =
            serde_json::from_str(config).unwrap();
        println!("{:?}", config);
        config
    });

/// 获取属性之间的关系
pub fn get_attribute_relationship(attribute: Attribute, target_attribute: Attribute) -> f32 {
    ATTRIBUTE_RELATIONSHIP
        .get(&attribute)
        .unwrap_or_else(|| panic!("Attribute {:?} not found", attribute))
        .get(&target_attribute)
        .unwrap_or(&NORMAL)
        .clone()
}
