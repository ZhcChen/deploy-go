//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'source_materialization_mode.g.dart';

class SourceMaterializationMode extends EnumClass {

  @BuiltValueEnumConst(wireName: r'full')
  static const SourceMaterializationMode full = _$full;
  @BuiltValueEnumConst(wireName: r'sparse')
  static const SourceMaterializationMode sparse = _$sparse;

  static Serializer<SourceMaterializationMode> get serializer => _$sourceMaterializationModeSerializer;

  const SourceMaterializationMode._(String name): super(name);

  static BuiltSet<SourceMaterializationMode> get values => _$values;
  static SourceMaterializationMode valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class SourceMaterializationModeMixin = Object with _$SourceMaterializationModeMixin;
