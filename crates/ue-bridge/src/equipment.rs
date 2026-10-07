//! Source node configuration selects equipment through BACC/BAD0 conditions.
use std::collections::{BTreeMap,BTreeSet};
use anyhow::{Context,ensure};
use tiger_pkg::{PackageManager,TagHash};
use crate::entity::{array,word,wide,relative,resource,source_identity,read_character};

pub fn read_equipment(manager:&PackageManager,table:TagHash,node_index:usize)->anyhow::Result<Vec<u8>> {
    ensure!(manager.get_entry(table).context("source node table missing")?.reference==0x8080B1A7,"node table class mismatch {table}");
    let data=manager.read_tag(table)?;
    let nodes=array(&data,8,0x90,0x8080B3F5)?;
    let node=*nodes.get(node_index).context("source node index outside table")?;
    let entity=source_identity(manager,&data,node+0x28)?;
    ensure!(manager.get_entry(entity).context("source node entity missing")?.reference==0x8080BAAD,"source node is not an entity {entity}");
    let mut config=BTreeMap::new();let mut classes=Vec::new();let mut visited=BTreeSet::new();
    let mut link=relative(&data,node+0x78)?;
    while let Some(p)=link {
        ensure!(visited.insert(p),"cyclic node component list {table}:{node_index}");
        let class=word(&data,p.checked_sub(4).context("node resource underflow")?)?;
        classes.push(format!("{class:08X}"));
        if class==0x8080402E {
            for pair in array(&data,p+0x10,8,0x8080402D)? {
                let key=word(&data,pair)?;let value=word(&data,pair+4)?;
                ensure!(config.insert(key,value).is_none(),"duplicate node configuration key {key:08X}");
            }
        }
        link=relative(&data,p)?;
    }
    ensure!(!config.is_empty(),"node {table}:{node_index} has no source configuration");
    let root=manager.read_tag(entity)?;let mut selected=Vec::new();let mut equipment_components=0;
    let mut attachment_names=BTreeMap::new();
    for p in array(&root,8,12,0x8080BAA2)? {
        let component=TagHash(word(&root,p)?);
        ensure!(manager.get_entry(component).context("equipment component missing")?.reference==0x8080BADB,"equipment component class mismatch {component}");
        let source=manager.read_tag(component)?;
        let Some((base,class))=resource(&source,0x18)? else {continue};
        if class==0x80809F76 {
            for row in array(&source,base+0xA8,48,0x80809F82)? {
                *attachment_names.entry(word(&source,row+0x28)?).or_insert(0usize)+=1;
            }
        }
        if class!=0x80804656 {continue;}
        equipment_components+=1;
        // The equipment transform input is a source attachment expression.
        // Verify its reciprocal instance/definition references and typed value;
        // do not guess a socket from mesh shape or hand-bone names.
        let (instance,instance_class)=resource(&source,0x10)?.context("equipment instance absent")?;
        ensure!(instance_class==0x80804655,"unsupported equipment instance class {component}");
        let binding=base+0x1B0;
        ensure!(word(&source,binding)?==component.0 && word(&source,binding+4)?==0x8080BA6E && word(&source,binding+8)? as usize==instance+0x50,
            "equipment transform definition reference mismatch {component}");
        ensure!(word(&source,instance+0x50)?==component.0 && word(&source,instance+0x54)?==0x8080BA6F && word(&source,instance+0x58)? as usize==binding,
            "equipment transform instance reference mismatch {component}");
        let expression=binding+0x10;
        ensure!(wide(&source,expression)?==0 && word(&source,expression+8)?==0x8080BA7A && word(&source,expression+0x10)?==0x26
            && word(&source,expression+0x20)?==0x8080BA9E,"unsupported source equipment transform expression {component}:{expression:X}");
        let attachment=word(&source,expression+0x24)?;
        let expression_fields=(0..12).map(|i|word(&source,expression+i*4)).collect::<anyhow::Result<Vec<_>>>()?;
        let expression_values=array(&source,expression+0x30,48,0x8080BA9E)?.into_iter()
            .map(|row|(0..12).map(|i|word(&source,row+i*4)).collect::<anyhow::Result<Vec<_>>>()).collect::<anyhow::Result<Vec<_>>>()?;
        let mut conditions=Vec::new();
        for row in array(&source,base+0x28,24,0x8080BACC)? {
            let pairs=array(&source,row+8,8,0x8080BAD0)?.into_iter()
                .map(|pair|Ok((word(&source,pair)?,word(&source,pair+4)?))).collect::<anyhow::Result<Vec<_>>>()?;
            conditions.push(pairs);
        }
        let mut matched=0;
        for row in array(&source,base+0x190,64,0x8080465E)? {
            let condition_index=word(&source,row+8)? as usize;
            let pairs=conditions.get(condition_index).context("equipment condition index outside table")?;
            ensure!(!pairs.is_empty(),"equipment option has no explicit source switch conditions {component}");
            if !pairs.iter().all(|(key,value)|config.get(key)==Some(value)) {continue;}
            matched+=1;
            let fields=(0..12).map(|i|word(&source,row+i*4)).collect::<anyhow::Result<Vec<_>>>()?;
            for item in array(&source,row+0x30,24,0x80804664)? {
                let child=source_identity(manager,&source,item+8)?;
                ensure!(child!=entity,"self-referential equipment entity {entity}");
                let graph:serde_json::Value=serde_json::from_slice(&read_character(manager,child)?)?;
                selected.push(serde_json::json!({"component":component.to_string(),"condition_index":condition_index,
                    "conditions":pairs,"source_fields":fields,"item_fields":[word(&source,item)?,word(&source,item+4)?],
                    "attachment_hash":format!("{attachment:08X}"),"attachment_expression":{"offset":expression,"operation":0x26,"value_class":"8080BA9E",
                        "source_fields":expression_fields,"source_values":expression_values},
                    "entity":child.to_string(),"assembly":graph}));
            }
        }
        ensure!(matched==1,"source equipment configuration matched {matched} options in {component}");
    }
    ensure!(equipment_components>0 && !selected.is_empty(),"node {table}:{node_index} selects no decoded source equipment");
    for item in &selected {
        let name=u32::from_str_radix(item["attachment_hash"].as_str().context("attachment hash absent")?,16)?;
        ensure!(attachment_names.get(&name)==Some(&1),"equipment attachment {name:08X} does not resolve uniquely on {entity}");
    }
    let config:Vec<_>=config.into_iter().collect();
    Ok(serde_json::to_vec(&serde_json::json!({"node_table":table.to_string(),"node_index":node_index,
        "entity":entity.to_string(),"configuration":config,"node_component_classes":classes,
        "world_identity":format!("{:016X}",wide(&data,node+0x70)? as u64),"equipment":selected}))?)
}
