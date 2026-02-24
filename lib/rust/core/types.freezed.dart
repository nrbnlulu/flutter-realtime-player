// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$VideoConfig {

 WscRtpSessionConfig get field0;
/// Create a copy of VideoConfig
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VideoConfigCopyWith<VideoConfig> get copyWith => _$VideoConfigCopyWithImpl<VideoConfig>(this as VideoConfig, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VideoConfig&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'VideoConfig(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $VideoConfigCopyWith<$Res>  {
  factory $VideoConfigCopyWith(VideoConfig value, $Res Function(VideoConfig) _then) = _$VideoConfigCopyWithImpl;
@useResult
$Res call({
 WscRtpSessionConfig field0
});




}
/// @nodoc
class _$VideoConfigCopyWithImpl<$Res>
    implements $VideoConfigCopyWith<$Res> {
  _$VideoConfigCopyWithImpl(this._self, this._then);

  final VideoConfig _self;
  final $Res Function(VideoConfig) _then;

/// Create a copy of VideoConfig
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? field0 = null,}) {
  return _then(_self.copyWith(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as WscRtpSessionConfig,
  ));
}

}


/// Adds pattern-matching-related methods to [VideoConfig].
extension VideoConfigPatterns on VideoConfig {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( VideoConfig_WscRtp value)?  wscRtp,required TResult orElse(),}){
final _that = this;
switch (_that) {
case VideoConfig_WscRtp() when wscRtp != null:
return wscRtp(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( VideoConfig_WscRtp value)  wscRtp,}){
final _that = this;
switch (_that) {
case VideoConfig_WscRtp():
return wscRtp(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( VideoConfig_WscRtp value)?  wscRtp,}){
final _that = this;
switch (_that) {
case VideoConfig_WscRtp() when wscRtp != null:
return wscRtp(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( WscRtpSessionConfig field0)?  wscRtp,required TResult orElse(),}) {final _that = this;
switch (_that) {
case VideoConfig_WscRtp() when wscRtp != null:
return wscRtp(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( WscRtpSessionConfig field0)  wscRtp,}) {final _that = this;
switch (_that) {
case VideoConfig_WscRtp():
return wscRtp(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( WscRtpSessionConfig field0)?  wscRtp,}) {final _that = this;
switch (_that) {
case VideoConfig_WscRtp() when wscRtp != null:
return wscRtp(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class VideoConfig_WscRtp extends VideoConfig {
  const VideoConfig_WscRtp(this.field0): super._();
  

@override final  WscRtpSessionConfig field0;

/// Create a copy of VideoConfig
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VideoConfig_WscRtpCopyWith<VideoConfig_WscRtp> get copyWith => _$VideoConfig_WscRtpCopyWithImpl<VideoConfig_WscRtp>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VideoConfig_WscRtp&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'VideoConfig.wscRtp(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $VideoConfig_WscRtpCopyWith<$Res> implements $VideoConfigCopyWith<$Res> {
  factory $VideoConfig_WscRtpCopyWith(VideoConfig_WscRtp value, $Res Function(VideoConfig_WscRtp) _then) = _$VideoConfig_WscRtpCopyWithImpl;
@override @useResult
$Res call({
 WscRtpSessionConfig field0
});




}
/// @nodoc
class _$VideoConfig_WscRtpCopyWithImpl<$Res>
    implements $VideoConfig_WscRtpCopyWith<$Res> {
  _$VideoConfig_WscRtpCopyWithImpl(this._self, this._then);

  final VideoConfig_WscRtp _self;
  final $Res Function(VideoConfig_WscRtp) _then;

/// Create a copy of VideoConfig
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(VideoConfig_WscRtp(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as WscRtpSessionConfig,
  ));
}


}

// dart format on
