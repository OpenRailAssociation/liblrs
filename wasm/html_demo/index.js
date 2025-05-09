'use strict';
import Bbox from '@turf/bbox';
import * as turf from '@turf/helpers';
import Alpine from 'alpinejs';
import * as maplibregl from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import { Protocol } from 'pmtiles';
import { LrmScaleMeasure, Lrs, Point, set_panic_hook } from '../pkg/liblrs_wasm';

// For the rust bindings: this allows us to have nice error messages
set_panic_hook()

async function file_selected(el) {
    const [file] = el.target.files;
    const data = await file.arrayBuffer()
    const lrs = await Lrs.load(new Uint8Array(data));

    const curves_features = []
    const anchors_features = []
    for (let lrm_index = 0; lrm_index < lrs.lrm_len(); lrm_index++) {
        const anchors = lrs.get_anchors(lrm_index)
        const lrm_id = lrs.get_lrm_scale_id(lrm_index);
        const geom = lrs.get_lrm_geom(lrm_index);
        const feature = turf.lineString(geom.map(p => [p.x, p.y]), { id: lrm_id, anchors }, {
            id: lrm_index,
        });
        feature.bbox = Bbox(feature)
        curves_features.push(feature);

        for (let anchor_index = 0; anchor_index < anchors.length; anchor_index++) {
            const anchor = anchors[anchor_index];
            anchor.properties = Object.fromEntries(lrs.anchor_properties(lrm_index, anchor_index))
            const properties = { id: anchor.name, lrm_id, curve: anchor.curve_position, scale: anchor.scale_position }
            if (anchor.position) {
                anchors_features.push(turf.point([anchor.position.x, anchor.position.y], properties, { id: anchors_features.length }))
            }
        }
    }

    map.addSource('lrms', {
        'type': 'geojson',
        'data': turf.featureCollection(curves_features),
    });

    map.addSource('anchors', {
        'type': 'geojson',
        'data': turf.featureCollection(anchors_features),
    })

    map.addSource('pr', {
        'type': 'geojson',
        'data': turf.featureCollection([])
    })

    map.addSource('range', {
        type: 'geojson',
        data: turf.featureCollection([])
    })

    map.addLayer({
        'id': 'lrms',
        'type': 'line',
        'source': 'lrms',
        'paint': {
            'line-color': '#888',
            'line-width': 2
        }
    });


    map.addLayer({
        'id': 'lrms-hitbox',
        'type': 'line',
        'source': 'lrms',
        'paint': {
            'line-width': 10,
            'line-opacity': 0
        }
    });

    map.addLayer({
        'id': 'lrms-hover',
        'type': 'line',
        'source': 'lrms',
        'paint': {
            'line-color': 'red',
            'line-width': 3,
            'line-opacity': [
                'case',
                ['boolean', ['feature-state', 'hover'], false],
                1,
                0
            ]
        }
    });

    map.addLayer({
        id: 'anchors-unselected',
        type: 'circle',
        source: 'anchors',
        paint: {
            'circle-radius': 5,
            'circle-color': '#eee',
            'circle-opacity': ['case',
                ['boolean', ['feature-state', 'selected'], true],
                0,
                1,
            ]
        }
    })

    map.addLayer({
        id: 'anchors',
        type: 'circle',
        source: 'anchors',
        paint: {
            'circle-radius': 5,
            'circle-color': 'blue',
            'circle-opacity': ['case',
                ['boolean', ['feature-state', 'selected'], true],
                1,
                0,
            ]
        }
    })

    map.addLayer({
        id: 'anchors-labels',
        type: 'symbol',
        source: 'anchors',
        layout: {
            //'text-field': ['concat', ['get', 'id'], ['literal', '-'], ['get', 'lrm_id'], ['literal', ' curve_pos:'], ['get', 'curve'], ['literal', ' scale:'], ['get', 'scale']],
            'text-field': ['get', 'id'],
            'text-offset': [1, 0],
        },
        paint: {
            'text-halo-color': 'white',
            'text-halo-width': 2,
        }
    })

    map.addLayer({
        id: 'pr-outline',
        type: 'circle',
        source: 'pr',
        paint: {
            'circle-radius': 10,
            'circle-color': 'white',
        }
    })

    map.addLayer({
        id: 'pr',
        type: 'circle',
        source: 'pr',
        paint: {
            'circle-radius': 6,
            'circle-color': 'red',
        }
    })

    map.addLayer({
        'id': 'range',
        'type': 'line',
        'source': 'range',
        'paint': {
            'line-color': 'yellow',
            'line-width': 2
        }
    });



    map.on('mouseenter', 'lrms-hitbox', () => {
        map.getCanvas().style.cursor = 'pointer'
    })
    map.on('mouseleave', 'lrms-hitbox', () => {
        map.getCanvas().style.cursor = ''
    })


    map.on('click', 'lrms-hitbox', (e) => {

        let lrm_id = e.features[0].id;
        let clicked_point = new Point(e.lngLat.lng, e.lngLat.lat);

        let projection = lrs.lookup(clicked_point, lrm_id)[0];

        let window_lrms = window.Alpine.store('lrms')
        if (window_lrms.lrm_id !== lrm_id) {
            window_lrms.details(lrm_id, false)
        }

        window_lrms.selectedFeature = curves_features[lrm_id];
        let offset = Math.round(projection.measure.scale_offset)
        window_lrms.pkStart = projection.measure.anchor_name + '+' + String(offset).padStart(3, "0");

        window_lrms.startMeasure = projection.measure;
        let point = lrs.resolve(lrm_id, projection.measure)
        window_lrms.pkStartPoint = turf.point([point.x, point.y]);
        window_lrms.handlePks(false)
    });

    return {
        features: curves_features,
        filename: file.name,
        filesize: data.byteLength,
        anchors_features,
        lrs,
    }
}

let protocol = new Protocol();
maplibregl.addProtocol('pmtiles', protocol.tile);

let map = new maplibregl.Map({
    container: 'map', // container id
    style: 'https://tuiles.enliberte.fr/styles/basic.json',
    center: [2.3469, 46.8589], // starting position [lng, lat]
    zoom: 5, // starting zoom,
});


window.Alpine = Alpine
Alpine.store('lrms', {
    status: 'waiting',
    error_text: null,
    lrms: [],
    lrm_id: null,
    lrs: null,
    filesize: null,
    filename: null,
    selectedFeature: null,
    pkStart: '',
    pkEnd: '',
    pkStartPoint: null,
    pkEndPoint: null,
    startMeasure: null,
    endMeasure: null,
    anchors: [],
    anchors_features: [],
    filter: '',

    async load(el) {
        try {
            this.status = 'loading'
            const lrs = await file_selected(el)
            this.lrms = lrs.features
            this.filename = lrs.filename
            this.filesize = lrs.filesize
            this.status = 'loaded'
            this.lrs = lrs.lrs
            this.anchors_features = lrs.anchors_features
        } catch (e) {
            this.status = 'error'
            this.error_text = e
        }

    },
    get waiting() { return this.status == 'waiting' },
    get loaded() { return this.status == 'loaded' },
    get error() { return this.status == 'error' },
    get loading() { return this.status == 'loading' },
    details(id, zoom = true) {
        this.unselect()
        this.lrm_id = id
        this.selectedFeature = this.lrms[id];

        map.setFeatureState(
            { source: 'lrms', id },
            { hover: true }
        )
        if (zoom) {
            map.fitBounds(this.lrms[id].bbox, { padding: 40 })
        }
        const lrm_id = this.lrms[id].properties.id

        for (const anchor of this.anchors_features) {
            if (anchor.properties.lrm_id !== lrm_id) {
                map.setFeatureState({ source: 'anchors', id: anchor.id }, { selected: false })
            }
        }
    },
    unselect() {
        if (this.selectedFeature) {
            map.setFeatureState(
                { source: 'lrms', id: this.selectedFeature.id },
                { hover: false }
            )
            this.selectedFeature = null
            for (const anchor of this.anchors_features) {
                map.setFeatureState({ source: 'anchors', id: anchor.id }, { selected: true })
            }
        }
    },
    startPkChange({ target }) {
        const re = /([0-9]+)\+([0-9]+)/;
        if (re.test(target.value)) {
            const [all, anchor_name, scale_offset] = target.value.match(re)
            this.startMeasure = new LrmScaleMeasure(anchor_name, scale_offset);
            const point = this.lrs.resolve(this.selectedFeature.id, this.startMeasure)
            this.pkStartPoint = turf.point([point.x, point.y]);
            this.handlePks()
        } else {
            this.pkStartPoint = null;
            this.startMeasure = null;
        }
    },
    endPkChange({ target }) {
        const re = /([0-9]+)\+([0-9]+)/;
        if (re.test(target.value)) {
            const [all, anchor_name, scale_offset] = target.value.match(re)
            this.endMeasure = new LrmScaleMeasure(anchor_name, scale_offset);
            const point = this.lrs.resolve(this.selectedFeature.id, this.endMeasure)
            this.pkEndPoint = turf.point([point.x, point.y])
            this.handlePks()
        } else {
            this.pkEndPoint = null;
            this.endMeasure = null;
        }
    },
    handlePks(move_window = true) {
        const points = [this.pkStartPoint, this.pkEndPoint].filter(p => p !== null)
        const geojson = turf.featureCollection(points);
        map.getSource('pr').setData(geojson);
        if (points.length === 1) {
            if (move_window) {
                map.flyTo({ center: points[0].geometry.coordinates, zoom: 15 })
            }
        } else {
            map.fitBounds(Bbox(geojson), { padding: 30 })
            const range = this.lrs.resolve_range(this.selectedFeature.id, this.startMeasure, this.endMeasure)
            const feature = turf.lineString(range.map(p => [p.x, p.y]));
            map.getSource('range').setData(turf.featureCollection([feature]))
        }
    }
})
Alpine.start()
