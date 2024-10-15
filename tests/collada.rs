use mesh_loader::collada::from_str;

// https://github.com/openrr/mesh-loader/issues/61
#[test]
fn issue61() {
    let cube = r##"<?xml version="1.0" encoding="utf-8"?>
    <COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <asset>
        <contributor>
        <author>Blender User</author>
        <authoring_tool>Blender 2.79.0 commit date:2018-03-22, commit time:14:10, hash:f4dc9f9d68b</authoring_tool>
        </contributor>
        <created>2018-11-19T22:54:36</created>
        <modified>2018-11-19T22:54:36</modified>
        <unit name="meter" meter="1"/>
        <up_axis>Z_UP</up_axis>
    </asset>
    <library_images/>
    <library_effects>
        <effect id="Material-effect">
        <profile_COMMON>
            <technique sid="common">
            <phong>
                <emission>
                <color sid="emission">0 0 0 1</color>
                </emission>
                <ambient>
                <color sid="ambient">0 0 0 1</color>
                </ambient>
                <diffuse>
                <color sid="diffuse">0.64 0.64 0.64 1</color>
                </diffuse>
                <specular>
                <color sid="specular">0.5 0.5 0.5 1</color>
                </specular>
                <shininess>
                <float sid="shininess">50</float>
                </shininess>
                <index_of_refraction>
                <float sid="index_of_refraction">1</float>
                </index_of_refraction>
            </phong>
            </technique>
        </profile_COMMON>
        </effect>
    </library_effects>
    <library_materials>
        <material id="Material-material" name="Material">
        <instance_effect url="#Material-effect"/>
        </material>
    </library_materials>
    <library_geometries>
        <geometry id="Cube_001-mesh" name="Cube.001">
        <mesh>
            <source id="Cube_001-mesh-positions">
            <float_array id="Cube_001-mesh-positions-array" count="24">-0.5 -0.5 -0.5 -0.5 -0.5 0.5 -0.5 0.5 -0.5 -0.5 0.5 0.5 0.5 -0.5 -0.5 0.5 -0.5 0.5 0.5 0.5 -0.5 0.5 0.5 0.5</float_array>
            <technique_common>
                <accessor source="#Cube_001-mesh-positions-array" count="8" stride="3">
                <param name="X" type="float"/>
                <param name="Y" type="float"/>
                <param name="Z" type="float"/>
                </accessor>
            </technique_common>
            </source>
            <source id="Cube_001-mesh-normals">
            <float_array id="Cube_001-mesh-normals-array" count="18">-1 0 0 0 1 0 1 0 0 0 -1 0 0 0 -1 0 0 1</float_array>
            <technique_common>
                <accessor source="#Cube_001-mesh-normals-array" count="6" stride="3">
                <param name="X" type="float"/>
                <param name="Y" type="float"/>
                <param name="Z" type="float"/>
                </accessor>
            </technique_common>
            </source>
            <vertices id="Cube_001-mesh-vertices">
            <input semantic="POSITION" source="#Cube_001-mesh-positions"/>
            </vertices>
            <triangles material="Material-material" count="12">
            <input semantic="VERTEX" source="#Cube_001-mesh-vertices" offset="0"/>
            <input semantic="NORMAL" source="#Cube_001-mesh-normals" offset="1"/>
            <p>1 0 2 0 0 0 3 1 6 1 2 1 7 2 4 2 6 2 5 3 0 3 4 3 6 4 0 4 2 4 3 5 5 5 7 5 1 0 3 0 2 0 3 1 7 1 6 1 7 2 5 2 4 2 5 3 1 3 0 3 6 4 4 4 0 4 3 5 1 5 5 5</p>
            </triangles>
        </mesh>
        </geometry>
    </library_geometries>
    <library_controllers/>
    <library_visual_scenes>
        <visual_scene id="Scene" name="Scene">
        <node id="Cube" name="Cube" type="NODE">
            <matrix sid="transform">1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1</matrix>
            <instance_geometry url="#Cube_001-mesh" name="Cube">
            <bind_material>
                <technique_common>
                <instance_material symbol="Material-material" target="#Material-material"/>
                </technique_common>
            </bind_material>
            </instance_geometry>
        </node>
        </visual_scene>
    </library_visual_scenes>
    <scene>
        <instance_visual_scene url="#Scene"/>
    </scene>
    </COLLADA>"##;

    let scene = from_str(cube).unwrap();
    assert_eq!(scene.materials[0].name, "Material");
}
