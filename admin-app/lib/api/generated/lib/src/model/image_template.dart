//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'image_template.g.dart';

class ImageTemplate extends EnumClass {

  @BuiltValueEnumConst(wireName: r'redis')
  static const ImageTemplate redis = _$redis;
  @BuiltValueEnumConst(wireName: r'valkey')
  static const ImageTemplate valkey = _$valkey;
  @BuiltValueEnumConst(wireName: r'postgres')
  static const ImageTemplate postgres = _$postgres;
  @BuiltValueEnumConst(wireName: r'etcd')
  static const ImageTemplate etcd = _$etcd;

  static Serializer<ImageTemplate> get serializer => _$imageTemplateSerializer;

  const ImageTemplate._(String name): super(name);

  static BuiltSet<ImageTemplate> get values => _$values;
  static ImageTemplate valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class ImageTemplateMixin = Object with _$ImageTemplateMixin;
