extern crate vpx_sys;

use std::{mem,ptr};
use vpx_sys::*;

fn get_packets(mut ctx: vpx_codec_ctx_t) -> Option<Vec<u8>> {    
    unsafe {
        let mut iter = mem::zeroed();

        loop {
            let pkt = vpx_codec_get_cx_data(&mut ctx, &mut iter);

            if pkt.is_null() {            
                break;
            } else {            
                println!("{:#?}", (*pkt).kind);
                
                if (*pkt).kind == vpx_codec_cx_pkt_kind::VPX_CODEC_CX_FRAME_PKT {    
                    //println!("{:#?}",(*pkt).data.frame);
                    let f = (*pkt).data.frame ;

                    println!("frame length: {} bytes", f.sz);

                    let mut image_frame: Vec<u8> = Vec::with_capacity(f.sz);                        
                    ptr::copy_nonoverlapping(mem::transmute(f.buf), image_frame.as_mut_ptr(), f.sz);
                    image_frame.set_len(f.sz);

                    return Some(image_frame);
                };                
            }
        };
    };

    None
}

fn encode_frame(mut ctx: vpx_codec_ctx_t, mut img: vpx_image, frame: i64, flags: i64) -> Result<Option<Vec<u8>>, vpx_codec_err_t> {
    let ret = unsafe {
             vpx_codec_encode(
                &mut ctx,
                &mut img,
                frame,
                1,
                flags,
                VPX_DL_GOOD_QUALITY as u64,
            )
    };

    match ret {
            VPX_CODEC_OK => {
                Ok(get_packets(ctx))
            },
            _ => Err(ret),
    }    
}

fn flush_frame(mut ctx: vpx_codec_ctx_t) -> Result<Option<Vec<u8>>, vpx_codec_err_t> {
    let ret = unsafe {        
        vpx_codec_encode(
        &mut ctx,
        ptr::null_mut(),
        -1,
        1,
        0,
        VPX_DL_GOOD_QUALITY as u64,
        )
    };

    match ret {
            VPX_CODEC_OK => Ok(get_packets(ctx)),
            _ => Err(ret),
    } 
}

fn main() {
    println!("VP9 encoding test");

    let w = 300 as u32 ;
    let h = 300 as u32 ;

    let mut raw: vpx_image = unsafe { mem::uninitialized() };
    let mut ctx: vpx_codec_ctx_t = unsafe { mem::uninitialized() };

    let align = 1 ;

    let ret = unsafe { vpx_img_alloc(&mut raw, vpx_img_fmt::VPX_IMG_FMT_I420, w, h, align) };//I420
    if ret.is_null() {
        println!("VP9 image frame error: image allocation failed");
        return ;
    }

    mem::forget(ret); // img and ret are the same
    print!("{:#?}", raw);

    let pixel_count = w * h ;
    let y : &[u8] = &vec![128; pixel_count as usize];
    let u : &[u8] = &vec![128; (pixel_count/4) as usize];
    let v : &[u8] = &vec![128; (pixel_count/4) as usize];        

    raw.planes[0] = unsafe { mem::transmute(y.as_ptr()) };
    raw.planes[1] = unsafe { mem::transmute(u.as_ptr()) };
    raw.planes[2] = unsafe { mem::transmute(v.as_ptr()) };

    let mut cfg = unsafe { mem::uninitialized() };
        let mut ret = unsafe { vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut cfg, 0) };

        if ret != VPX_CODEC_OK {
            println!("VP9 image frame error: default Configuration failed");

            //release the image
            unsafe {
                vpx_img_free(&mut raw)
            };

            return ;
        }

        cfg.g_w = w;
        cfg.g_h = h;
        cfg.g_timebase.num = 1;
        cfg.g_timebase.den = 30;
        cfg.rc_target_bitrate = 100 * 1014;

        ret = unsafe {
            vpx_codec_enc_init_ver(
                &mut ctx,
                vpx_codec_vp9_cx(),
                &mut cfg,
                0,
                (14+4+5) as i32,//23 for libvpx-1.7.0; VPX_ENCODER_ABI_VERSION does not get expanded correctly by bind-gen
            )
        };

        if ret != VPX_CODEC_OK {            
            println!("VP9 image frame error: codec init failed {:?}", ret);

            unsafe {
                vpx_img_free(&mut raw)
            };  

            return ;
        }

    let mut image_frame : Vec<u8> = Vec::new() ;

    let mut flags = 0;
    flags |= VPX_EFLAG_FORCE_KF;

    //call encode_frame with a valid image
    match encode_frame(ctx, raw, 0, flags as i64) {
        Ok(res) => match res {
            Some(res) => image_frame = res,
            _ => {},
        },
        Err(err) => println!("codec error: {:?}", err),
    }; 

    //flush the encoder to signal the end    
    match flush_frame(ctx) {
        Ok(res) => match res {
            Some(res) => image_frame = res,
            _ => {},
        },
        Err(err) => println!("codec error: {:?}", err),
    }; 

    println!("{:?}", image_frame);

    //release memory
    unsafe {
        vpx_img_free(&mut raw)
    };

    unsafe {
        vpx_codec_destroy(&mut ctx)
    };
}
