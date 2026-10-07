//! Marathon clip streams. No retargeting, replacement poses or implicit channels.
//! Binary format reference: SolUnshadowed/tiger-animation-parser b9fdc3a.
//! Implementation uses checked byte access and a common channel evaluator.
use anyhow::{Context, Result, ensure, bail};
use serde::Serialize;
use tiger_pkg::{PackageManager, TagHash};

struct View<'a>(&'a [u8]);
impl View<'_> {
    fn bytes(&self, p: usize, n: usize) -> Result<&[u8]> {
        self.0.get(p..p.checked_add(n).context("clip offset overflow")?).context("clip read outside entry")
    }
    fn h(&self,p:usize)->Result<u16>{Ok(u16::from_le_bytes(self.bytes(p,2)?.try_into()?))}
    fn w(&self,p:usize)->Result<u32>{Ok(u32::from_le_bytes(self.bytes(p,4)?.try_into()?))}
    fn q(&self,p:usize)->Result<u64>{Ok(u64::from_le_bytes(self.bytes(p,8)?.try_into()?))}
    fn f(&self,p:usize)->Result<f32>{let f=f32::from_bits(self.w(p)?);ensure!(f.is_finite(),"nonfinite clip value");Ok(f)}
    fn rel(&self,p:usize)->Result<Option<usize>> {
        let d=self.q(p)? as i64;
        if d==0{return Ok(None)}
        let target=(p as i64).checked_add(d).context("clip pointer overflow")?;
        ensure!(target>=0 && (target as usize)<self.0.len(),"clip pointer outside entry");
        Ok(Some(target as usize))
    }
    fn array(&self,p:usize,width:usize)->Result<&[u8]> {
        let n=usize::try_from(self.q(p)?)?;
        if n==0{return Ok(&[])}
        let h=self.rel(p+8)?.context("null nonempty clip array")?;
        ensure!(self.q(h)?==n as u64,"clip array count mismatch");
        self.bytes(h+16,n.checked_mul(width).context("clip array overflow")?)
    }
}

#[derive(Serialize)]
pub struct Channel {
    pub control: u16,
    pub kind: &'static str,
    pub constant: bool,
    pub samples: Vec<Vec<f32>>,
}
#[derive(Serialize)]
pub struct Clip {
    pub tag: String,
    pub name_hash: String,
    pub frames: u16,
    pub nodes: u16,
    pub controls: u16,
    pub subsets: Vec<[u32;2]>,
    pub channels: Vec<Channel>,
}

fn half_array(b:&[u8])->Vec<u16>{b.chunks_exact(2).map(|v|u16::from_le_bytes([v[0],v[1]])).collect()}
fn float_array(b:&[u8])->Result<Vec<f32>> {
    b.chunks_exact(4).map(|b|{let f=f32::from_le_bytes(b.try_into()?);ensure!(f.is_finite(),"nonfinite quantization");Ok(f)}).collect()
}
fn snorm(v:u16)->f32{(v as i16) as f32/32767.0}
fn signed_index(v:u16)->Result<usize>{let n=(v as i16) as i32;ensure!(n!=0,"zero clip stream locator");Ok((n.abs()-1) as usize)}

// Components of codec-1 quaternions store w,x,y; x's low bit carries z sign.
fn sample(raw:&[u16], kind:usize, quant:[f32;2])->Result<Vec<f32>> {
    match kind {
        0=>Ok(vec![snorm(*raw.first().context("missing scale sample")?)]),
        1=>{
            ensure!(raw.len()==3,"invalid quaternion sample");
            let w=snorm(raw[0]);let x=snorm(raw[1]);let y=snorm(raw[2]);
            let z=(1.0-w*w-x*x-y*y).max(0.0).sqrt()*if raw[1]&1==0{1.0}else{-1.0};
            Ok(vec![x,y,z,w])
        }
        2=>Ok(raw.iter().map(|&v|snorm(v)*quant[0]+quant[1]).collect()),
        _=>bail!("invalid channel kind"),
    }
}
fn tangent(n:u8, delta:f32)->f32{let t=(n as f32-7.0)/7.0;delta+0.3*t*t.abs()}
fn curve(a:f32,b:f32, packed:u8,t:f32)->f32 {
    let t2=t*t;let t3=t2*t;let delta=b-a;
    (2.0*t3-3.0*t2+1.0)*a+(t3-2.0*t2+t)*tangent(packed>>4,delta)
        +(3.0*t2-2.0*t3)*b+(t3-t2)*tangent(packed&15,delta)
}

fn codec(v:&View, p:usize, frames:usize, maps:[Vec<u16>;3], constant:bool)->Result<Vec<Channel>> {
    let counts=[v.h(p+2)? as usize,v.h(p+4)? as usize,v.h(p+6)? as usize];
    for k in 0..3 {ensure!(counts[k]==maps[k].len(),"clip map/stream count mismatch");}
    let mut channels=Vec::new();
    let names=["scale","rotation","translation"];
    match v.h(p)? {
        3=>{
            let count=v.w(p+16)? as usize;
            ensure!(count==if constant{1}else{frames},"codec-3 frame count mismatch");
            let raw=half_array(v.array(p+56,2)?);
            let widths=[1,4,3];let mut cursor=0;
            for k in 0..3 {for &control in &maps[k] {
                let mut samples=Vec::with_capacity(count);
                for _ in 0..count {
                    let mut value=Vec::with_capacity(widths[k]);
                    for c in 0..widths[k] {
                        let u=*raw.get(cursor).context("truncated codec-3 stream")? as f32/65535.0;cursor+=1;
                        value.push(match k {
                            0=>u*v.f(p+20)?+v.f(p+24)?,
                            1=>u*2.0-1.0,
                            2=>u*v.f(p+28+c*4)?+v.f(p+40+c*4)?,
                            _=>unreachable!(),
                        });
                    }
                    samples.push(value);
                }
                channels.push(Channel{control,kind:names[k],constant,samples});
            }}
            ensure!(cursor==raw.len(),"unconsumed codec-3 stream");
        }
        1=>{
            ensure!(!constant,"static codec-1 unsupported");
            let direct=half_array(v.array(p+16,2)?);
            let keys=half_array(v.array(p+32,2)?);
            let intervals=v.array(p+48,1)?;
            let slopes=v.array(p+64,1)?;
            let factors=float_array(v.array(p+80,4)?)?;
            let biases=float_array(v.array(p+96,4)?)?;
            let locators=half_array(v.array(p+112,2)?);
            ensure!(locators.len()==counts.iter().sum::<usize>()+1,"clip locator table size mismatch");
            ensure!(locators.last().copied()==Some(keys.len() as u16),"clip locator end mismatch");
            ensure!(factors.len()==counts[2] && biases.len()==counts[2],"translation quantization count mismatch");
            let mut locator=0;
            for k in 0..3 {for (index,&control) in maps[k].iter().enumerate() {
                let code=locators[locator];locator+=1;
                let width=if k==0{1}else{3};
                let quant=if k==2{[factors[index],biases[index]]}else{[1.0,0.0]};
                let mut position=signed_index(code)?;
                let mut samples=Vec::with_capacity(frames);
                if (code as i16) < 0 {
                    for _ in 0..frames {
                        let raw=direct.get(position..position+width).context("truncated direct clip stream")?;
                        samples.push(sample(raw,k,quant)?);position+=width;
                    }
                } else {
                    let header=keys.get(position..position+3).context("truncated curve header")?;position+=3;
                    let delta_start=header[0] as usize;let mut slope=header[1] as usize;let segments=header[2] as usize;
                    let mut previous=sample(keys.get(position..position+width).context("missing first curve key")?,k,quant)?;position+=width;
                    samples.push(previous.clone());
                    for s in 0..segments {
                        let step=*intervals.get(delta_start+s).context("curve interval outside table")? as usize;
                        ensure!(step>0 && samples.len()+step<=frames,"invalid curve interval");
                        let next=sample(keys.get(position..position+width).context("truncated curve keys")?,k,quant)?;position+=width;
                        let packed=slopes.get(slope..slope+previous.len()).context("curve tangent outside table")?;slope+=previous.len();
                        for tick in 1..=step {
                            let t=tick as f32/step as f32;
                            samples.push(previous.iter().zip(&next).zip(packed).map(|((&a,&b),&s)|curve(a,b,s,t)).collect());
                        }
                        previous=next;
                    }
                    ensure!(samples.len()==frames,"curve timeline does not cover clip");
                }
                channels.push(Channel{control,kind:names[k],constant,samples});
            }}
        }
        id=>bail!("unsupported Marathon animation codec {id}"),
    }
    ensure!(channels.iter().all(|c|c.samples.iter().flatten().all(|v|v.is_finite())),"nonfinite decoded animation");
    Ok(channels)
}

pub fn read_clip(manager:&PackageManager,tag:TagHash)->Result<Vec<u8>> {
    ensure!(manager.get_entry(tag).context("clip missing")?.reference==0x8080AE01,"animation class mismatch {tag}");
    let raw=manager.read_tag(tag)?;let v=View(&raw);
    ensure!(v.q(0)? as usize==raw.len(),"clip entry size mismatch");
    let frames=v.h(0x140)?;let nodes=v.h(0x142)?;let controls=v.h(0x144)?;
    ensure!(frames>0 && controls>0,"empty clip");
    let mut channels=Vec::new();
    for (i,field) in [0x10,0x18].into_iter().enumerate() {
        if let Some(p)=v.rel(field)? {
            let mut maps=[Vec::new(),Vec::new(),Vec::new()];
            for k in 0..3 {
                maps[k]=half_array(v.array(0xA8+(i*3+k)*16,2)?);
                ensure!(maps[k].iter().all(|&c|c<controls),"clip control index outside rig subset");
            }
            channels.extend(codec(&v,p,frames as usize,maps,i==0)?);
        }
    }
    let subsets=v.array(0x170,8)?.chunks_exact(8).map(|b|Ok([u32::from_le_bytes(b[..4].try_into()?),u32::from_le_bytes(b[4..].try_into()?)]))
        .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::to_vec(&Clip{tag:tag.to_string(),name_hash:format!("{:08X}",v.w(0x120)?),frames,nodes,controls,subsets,channels})?)
}
